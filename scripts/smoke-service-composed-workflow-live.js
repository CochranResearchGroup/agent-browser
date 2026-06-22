#!/usr/bin/env node

import { createServer } from 'node:http';
import { mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { runComposedWorkflow } from '../examples/service-client/composed-workflow.mjs';
import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';

const context = createSmokeContext({
  prefix: 'ab-service-composed-workflow-',
  sessionPrefix: 'service-composed-workflow',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'ServiceComposedWorkflowSmoke';
const agentName = 'smoke-agent';
const taskName = 'plan0034ComposedWorkflow';
const targetServiceId = 'generic-composed-site';
const accountId = 'composed-account@example.test';
const browserId = `session:${session}`;
const uploadDir = join(context.tempHome, 'upload');
const downloadDir = join(context.tempHome, 'downloads');
const uploadPath = join(uploadDir, 'upload-fixture.txt');
const downloadText = 'composed-download-fixture\n';
const timeout = setTimeout(() => {
  fail('Timed out waiting for service composed workflow live smoke to complete');
}, 180000);

let streamPort;
let server;
let serverUrl;

async function startServer() {
  server = createServer((req, res) => {
    const url = new URL(req.url ?? '/', 'http://127.0.0.1');
    if (url.pathname === '/') {
      res.writeHead(200, { 'content-type': 'text/html' });
      res.end(`<!doctype html>
<html>
  <head><title>Plan 0034 Composed Workflow</title></head>
  <body>
    <main data-account="${accountId}">Signed in as ${accountId}</main>
    <form>
      <label for="report-file">Upload report</label>
      <input id="report-file" type="file" />
      <label for="query">Query</label>
      <input id="query" name="query" />
      <button id="apply" type="button">Apply</button>
      <p id="status">Waiting</p>
    </form>
    <a id="download" href="/download/composed-download.txt" download="composed-download.txt">Download</a>
    <script>
      window.__composedIdentity = {
        detectedIdentity: "${accountId}",
        accountId: "${accountId}",
        confidence: "high"
      };
      document.querySelector('#apply').addEventListener('click', () => {
        document.querySelector('#status').textContent = 'Applied service text';
      });
      fetch('/api/data')
        .then((res) => res.json())
        .then((data) => {
          document.body.dataset.apiKind = data.kind;
        });
    </script>
  </body>
</html>`);
      return;
    }
    if (url.pathname === '/api/data') {
      res.writeHead(200, {
        'content-type': 'application/json',
        'x-smoke-allowed': 'composed',
      });
      res.end(JSON.stringify({ ok: true, kind: 'composed' }));
      return;
    }
    if (url.pathname === '/download/composed-download.txt') {
      res.writeHead(200, {
        'content-type': 'text/plain',
        'content-disposition': 'attachment; filename="composed-download.txt"',
      });
      res.end(downloadText);
      return;
    }
    res.writeHead(404, { 'content-type': 'text/plain' });
    res.end('not found');
  });
  server.keepAliveTimeout = 100;
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const address = server.address();
  serverUrl = `http://127.0.0.1:${address.port}`;
}

async function cleanup() {
  clearTimeout(timeout);
  if (server) {
    server.closeAllConnections?.();
    await new Promise((resolve) => server.close(resolve));
  }
  await closeSession(context);
  context.cleanupTempHome();
}

async function fail(message) {
  console.error(message);
  await cleanup();
  process.exit(1);
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
  for (const action of [
    'tab_new',
    'cdp_attach',
    'probe',
    'ui_action',
    'network_capture',
    'file_transfer',
    'diagnostics',
    'cdp_detach',
  ]) {
    assert(actions.has(action), `service trace missing ${action} job: ${JSON.stringify(trace?.jobs ?? [])}`);
  }
  assert((trace?.matched?.jobs ?? 0) >= 8, `service trace matched job count is too low: ${JSON.stringify(trace)}`);
}

try {
  mkdirSync(uploadDir, { recursive: true });
  mkdirSync(downloadDir, { recursive: true });
  writeFileSync(uploadPath, 'upload fixture\n');
  await startServer();
  streamPort = await ensureStreamPort(context, 120000);

  const result = await runComposedWorkflow({
    baseUrl: `http://127.0.0.1:${streamPort}`,
    url: `${serverUrl}/`,
    serviceName,
    agentName,
    taskName,
    loginId: targetServiceId,
    targetServiceId,
    accountId,
    tabParams: {
      headless: true,
      waitUntil: 'load',
    },
    timeoutMs: 10000,
    maxReturnBytes: 512,
    maxTextBytes: 256,
    maxBodyBytes: 512,
    uploadFile: uploadPath,
    uploadAllowedPath: uploadDir,
    downloadDir,
    downloadAllowedDir: context.tempHome,
  });

  const handle = result.serviceTabHandle;
  assert(handle?.valid === true, `workflow did not return a valid handle: ${JSON.stringify(handle)}`);
  assert(handle?.browserId === browserId, `serviceTabHandle browser mismatch: ${JSON.stringify(handle)}`);
  assert(result.attach?.attached === true, `workflow did not attach: ${JSON.stringify(result.attach)}`);
  assert(result.probe?.ok === true, `workflow probe was not ok: ${JSON.stringify(result.probe)}`);
  assert(
    result.probe?.identity?.detectedIdentity === accountId,
    `workflow identity mismatch: ${JSON.stringify(result.probe?.identity)}`,
  );
  assert(result.uiAction?.ok === true, `workflow ui_action was not ok: ${JSON.stringify(result.uiAction)}`);
  assert(result.networkCapture?.ok === true, `workflow network_capture was not ok: ${JSON.stringify(result.networkCapture)}`);
  assert(
    result.networkCapture?.events?.some((event) => event.url?.includes('/api/data')),
    `workflow network_capture missed /api/data: ${JSON.stringify(result.networkCapture?.events)}`,
  );
  assert(result.fileTransfer?.ok === true, `workflow file_transfer was not ok: ${JSON.stringify(result.fileTransfer)}`);
  assert(result.fileTransfer.upload?.uploaded === 1, `workflow upload count mismatch: ${JSON.stringify(result.fileTransfer.upload)}`);
  assert(
    result.fileTransfer.upload?.selectedFileNames?.includes('upload-fixture.txt'),
    `workflow selected file names mismatch: ${JSON.stringify(result.fileTransfer.upload)}`,
  );
  assert(
    result.fileTransfer.download?.fileName === 'composed-download.txt',
    `workflow download filename mismatch: ${JSON.stringify(result.fileTransfer.download)}`,
  );
  assert(readFileSync(result.fileTransfer.download.localPath, 'utf8') === downloadText, 'workflow download content mismatch');
  assert(statSync(result.fileTransfer.download.localPath).size <= 1024, 'workflow download exceeded cap');
  assert(result.diagnostics?.ok === true, `workflow diagnostics was not ok: ${JSON.stringify(result.diagnostics)}`);
  assert(
    result.diagnostics?.serviceTabHandle?.tabId === handle.tabId,
    `workflow diagnostics handle mismatch: ${JSON.stringify(result.diagnostics)}`,
  );
  assert(result.detach?.detached === true, `workflow did not detach: ${JSON.stringify(result.detach)}`);

  assertTraceJobs(await serviceTrace());

  await cleanup();
  console.log(`Service composed workflow live smoke passed (${browserId}, stream ${streamPort})`);
} catch (err) {
  await fail(err.stack || err.message);
}
