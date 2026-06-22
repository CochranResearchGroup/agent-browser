#!/usr/bin/env node

import { createServer, request as httpRequest } from 'node:http';
import { mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';

const context = createSmokeContext({
  prefix: 'ab-service-file-transfer-',
  sessionPrefix: 'service-file-transfer',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'ServiceFileTransferSmoke';
const agentName = 'smoke-agent';
const taskName = 'plan0034FileTransfer';
const targetServiceId = 'generic-file-transfer-site';
const browserId = `session:${session}`;
const uploadDir = join(context.tempHome, 'upload');
const downloadDir = join(context.tempHome, 'downloads');
const uploadPath = join(uploadDir, 'upload-fixture.txt');
const downloadText = 'generated-download-fixture\n';
const timeout = setTimeout(() => {
  fail('Timed out waiting for service file transfer live smoke to complete');
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
  <head><title>Plan 0034 File Transfer</title></head>
  <body>
    <main>
      <label for="report-file">Upload report</label>
      <input id="report-file" type="file" />
      <a id="download" href="/download/generated.txt" download="generated.txt">Download</a>
    </main>
  </body>
</html>`);
      return;
    }
    if (req.url === '/download/generated.txt') {
      res.writeHead(200, {
        'content-type': 'text/plain',
        'content-disposition': 'attachment; filename="generated.txt"',
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
  mkdirSync(uploadDir, { recursive: true });
  mkdirSync(downloadDir, { recursive: true });
  writeFileSync(uploadPath, 'upload fixture\n');
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

  const transferResponse = await serviceRequest(
    {
      action: 'file_transfer',
      serviceTabHandle: handle,
      timeoutMs: 10000,
      fileTransfer: {
        recipeId: 'upload-and-download',
        upload: {
          labelText: 'Upload report',
          files: [uploadPath],
          allowedPaths: [uploadDir],
          maxFiles: 1,
          verifySelectedNames: true,
        },
        download: {
          selector: '#download',
          directory: downloadDir,
          allowedDirectories: [context.tempHome],
          expectedFileName: 'generated.txt',
          maxBytes: 1024,
        },
      },
    },
    'file_transfer',
  );
  const transfer = transferResponse.data;
  assert(transfer?.ok === true, `file transfer was not ok: ${JSON.stringify(transferResponse)}`);
  assert(transfer.upload?.uploaded === 1, `upload count mismatch: ${JSON.stringify(transfer.upload)}`);
  assert(
    transfer.upload?.selectedFileNames?.includes('upload-fixture.txt'),
    `selected file names mismatch: ${JSON.stringify(transfer.upload)}`,
  );
  assert(transfer.download?.fileName === 'generated.txt', `download filename mismatch: ${JSON.stringify(transfer.download)}`);
  assert(transfer.download?.mimeType === 'text/plain', `download MIME mismatch: ${JSON.stringify(transfer.download)}`);
  assert(transfer.download?.sourceUrl?.includes('/download/generated.txt'), `download source URL mismatch: ${JSON.stringify(transfer.download)}`);
  assert(transfer.download?.size === Buffer.byteLength(downloadText), `download size mismatch: ${JSON.stringify(transfer.download)}`);
  assert(readFileSync(transfer.download.localPath, 'utf8') === downloadText, 'download content mismatch');
  assert(statSync(transfer.download.localPath).size <= 1024, 'download exceeded maxBytes');

  const failedResponse = await serviceRequest(
    {
      action: 'file_transfer',
      serviceTabHandle: handle,
      timeoutMs: 2000,
      captureEvidenceOnFailure: true,
      fileTransfer: {
        recipeId: 'missing-input',
        upload: {
          selector: '#missing-file-input',
          files: [uploadPath],
          allowedPaths: [uploadDir],
          maxFiles: 1,
        },
      },
    },
    'failing file_transfer',
  );
  const failed = failedResponse.data;
  assert(failed?.ok === false, `failing file transfer should return ok=false data: ${JSON.stringify(failedResponse)}`);
  assert(failed.failedPhase === 'upload', `failure phase mismatch: ${JSON.stringify(failed)}`);
  assert(failed.diagnostics?.ok === true, `failure diagnostics missing: ${JSON.stringify(failed)}`);
  assert(failed.diagnostics?.url?.includes(serverUrl), `diagnostics URL mismatch: ${JSON.stringify(failed.diagnostics)}`);

  const trace = await serviceTrace();
  const jobs = trace.jobs ?? [];
  const fileTransferJobs = jobs.filter((job) => job.action === 'file_transfer');
  assert(
    fileTransferJobs.length >= 2,
    `service trace missing file_transfer jobs: ${JSON.stringify(jobs)}`,
  );
  assert(
    fileTransferJobs.every((job) => job.state === 'succeeded'),
    `service trace missing successful file_transfer job: ${JSON.stringify(jobs)}`,
  );

  await cleanup();
  console.log(JSON.stringify({ success: true, uploadPath, downloadDir }));
} catch (err) {
  await fail(err.stack || err.message);
}
