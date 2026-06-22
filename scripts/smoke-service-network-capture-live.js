#!/usr/bin/env node

import { createServer, request as httpRequest } from 'node:http';

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';

const context = createSmokeContext({
  prefix: 'ab-service-network-capture-',
  sessionPrefix: 'service-network-capture',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'ServiceNetworkCaptureSmoke';
const agentName = 'smoke-agent';
const taskName = 'plan0034NetworkCapture';
const targetServiceId = 'generic-network-site';
const browserId = `session:${session}`;
const timeout = setTimeout(() => {
  fail('Timed out waiting for service network capture live smoke to complete');
}, 180000);

let streamPort;
let server;
let serverUrl;

async function startServer() {
  server = createServer((req, res) => {
    if (req.url === '/') {
      res.writeHead(200, { 'content-type': 'text/html' });
      res.end(`<!doctype html>
<html>
  <head><title>Plan 0034 Network Capture</title></head>
  <body>
    <main>Network capture smoke</main>
    <script>
      Promise.all([
        fetch('/api/small').then((res) => res.json()),
        fetch('/api/large').then((res) => res.text())
      ]).then(() => {
        document.body.dataset.ready = 'true';
      });
    </script>
  </body>
</html>`);
      return;
    }
    if (req.url === '/api/small') {
      res.writeHead(200, {
        'content-type': 'application/json',
        'x-smoke-allowed': 'small',
        'x-secret-token': 'do-not-return',
      });
      res.end(JSON.stringify({ ok: true, kind: 'small' }));
      return;
    }
    if (req.url === '/api/large') {
      res.writeHead(200, {
        'content-type': 'text/plain',
        'x-smoke-allowed': 'large',
      });
      res.end('large-body-'.repeat(80));
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

function httpJsonWithTimeout(port, method, path, body, timeoutMs) {
  return new Promise((resolve, reject) => {
    const rawBody = body === undefined ? undefined : JSON.stringify(body);
    const req = httpRequest(
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
  await startServer();
  streamPort = await ensureStreamPort(context, 120000);

  const tabResponse = await serviceRequest(
    {
      action: 'tab_new',
      params: {
        headless: true,
        url: `${serverUrl}/`,
        waitUntil: 'load',
      },
    },
    'tab_new',
  );
  const handle = tabResponse.data?.serviceTabHandle;
  assert(handle?.valid === true, `tab_new did not return a valid handle: ${JSON.stringify(tabResponse)}`);
  assert(handle?.browserId === browserId, `serviceTabHandle browser mismatch: ${JSON.stringify(handle)}`);

  const metadataResponse = await serviceRequest(
    {
      action: 'network_capture',
      serviceTabHandle: handle,
      timeoutMs: 5000,
      networkCapture: {
        recipeId: 'metadata-only',
        urlPatterns: ['/api/small'],
        methods: ['GET'],
        status: '2xx',
        maxEvents: 1,
        includeResponseHeaders: true,
        allowedHeaderNames: ['content-type'],
        trigger: { type: 'reload' },
      },
    },
    'metadata network_capture',
  );
  const metadata = metadataResponse.data;
  assert(metadata?.ok === true, `metadata capture was not ok: ${JSON.stringify(metadataResponse)}`);
  assert(metadata?.networkCapture?.metadataOnly === true, `metadata capture did not default to metadata-only: ${JSON.stringify(metadata)}`);
  assert(metadata?.events?.length === 1, `metadata capture event count mismatch: ${JSON.stringify(metadata?.events)}`);
  assert(metadata.events[0].url.includes('/api/small'), `metadata capture wrong URL: ${JSON.stringify(metadata.events[0])}`);
  assert(metadata.events[0].body?.captured === false, `metadata capture unexpectedly included body: ${JSON.stringify(metadata.events[0])}`);
  assert(metadata.events[0].headersRedacted === true, `metadata capture should mark headers redacted: ${JSON.stringify(metadata.events[0])}`);
  assert(metadata.events[0].responseHeaders?.['content-type'], `metadata capture missing allowlisted header: ${JSON.stringify(metadata.events[0])}`);
  assert(!metadata.events[0].responseHeaders?.['x-secret-token'], `metadata capture leaked non-allowlisted header: ${JSON.stringify(metadata.events[0])}`);

  const bodyResponse = await serviceRequest(
    {
      action: 'network_capture',
      serviceTabHandle: handle,
      timeoutMs: 5000,
      networkCapture: {
        recipeId: 'body-capture',
        urlPatterns: ['/api/small', '/api/large'],
        methods: ['GET'],
        status: '2xx',
        maxEvents: 2,
        captureBodies: true,
        maxBodyBytes: 32,
        trigger: { type: 'reload' },
      },
    },
    'body network_capture',
  );
  const bodyCapture = bodyResponse.data;
  assert(bodyCapture?.ok === true, `body capture was not ok: ${JSON.stringify(bodyResponse)}`);
  assert(bodyCapture?.events?.length === 2, `body capture event count mismatch: ${JSON.stringify(bodyCapture?.events)}`);
  const small = bodyCapture.events.find((event) => event.url.includes('/api/small'));
  const large = bodyCapture.events.find((event) => event.url.includes('/api/large'));
  assert(small?.body?.captured === true, `small response body was not captured: ${JSON.stringify(bodyCapture.events)}`);
  assert(String(small.body.body).includes('"kind":"small"'), `small response body content mismatch: ${JSON.stringify(small)}`);
  assert(large?.body?.captured === true, `large response body was not captured: ${JSON.stringify(bodyCapture.events)}`);
  assert(large.body.bodyTruncated === true, `large response body was not truncated: ${JSON.stringify(large)}`);
  assert(String(large.body.body).length <= 32, `large response body exceeded cap: ${JSON.stringify(large)}`);

  const trace = await serviceTrace();
  const networkJobs = (trace?.jobs ?? []).filter((job) => job.action === 'network_capture');
  assert(networkJobs.length >= 2, `service trace missing network_capture jobs: ${JSON.stringify(trace?.jobs ?? [])}`);

  await cleanup();
  console.log(`Service network capture live smoke passed (${browserId}, stream ${streamPort}, server ${serverUrl})`);
} catch (err) {
  await fail(err.stack || err.message);
}
