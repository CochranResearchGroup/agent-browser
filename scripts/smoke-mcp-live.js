#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const rootDir = new URL('..', import.meta.url).pathname;
const tempHome = mkdtempSync(join(tmpdir(), 'agent-browser-mcp-live-'));
const realHome = process.env.HOME;
const cargoHome = process.env.CARGO_HOME || (realHome ? join(realHome, '.cargo') : undefined);
const rustupHome = process.env.RUSTUP_HOME || (realHome ? join(realHome, '.rustup') : undefined);
const session = `mcp-live-${process.pid}-${Date.now()}`;
const agentHome = join(tempHome, '.agent-browser');
const socketDir = join(agentHome, 'sockets');
const profileDir = join(tempHome, 'chrome-profile');
const serviceName = 'McpLiveSmoke';
const agentName = 'smoke-agent';
const taskName = 'browserSnapshotSmoke';

mkdirSync(socketDir, { recursive: true });
mkdirSync(profileDir, { recursive: true });

const env = {
  ...process.env,
  HOME: tempHome,
  AGENT_BROWSER_HOME: agentHome,
  AGENT_BROWSER_SOCKET_DIR: socketDir,
  AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0',
  ...(cargoHome ? { CARGO_HOME: cargoHome } : {}),
  ...(rustupHome ? { RUSTUP_HOME: rustupHome } : {}),
};

let child;
let stdout = '';
let stderr = '';
let nextId = 1;
const pending = new Map();
const timeout = setTimeout(() => {
  fail('Timed out waiting for live MCP smoke to complete');
}, 90000);

function cargoArgs(args) {
  return ['run', '--quiet', '--manifest-path', 'cli/Cargo.toml', '--', ...args];
}

function runCli(args) {
  return new Promise((resolve, reject) => {
    const proc = spawn('cargo', cargoArgs(args), {
      cwd: rootDir,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const procTimeout = setTimeout(() => {
      proc.kill('SIGTERM');
      reject(new Error(`agent-browser ${args.join(' ')} timed out`));
    }, 60000);
    let out = '';
    let err = '';
    proc.stdout.setEncoding('utf8');
    proc.stderr.setEncoding('utf8');
    proc.stdout.on('data', (chunk) => {
      out += chunk;
    });
    proc.stderr.on('data', (chunk) => {
      err += chunk;
    });
    proc.on('error', (err) => {
      clearTimeout(procTimeout);
      reject(err);
    });
    proc.on('exit', (code, signal) => {
      clearTimeout(procTimeout);
      if (code === 0) {
        resolve({ stdout: out, stderr: err });
      } else {
        reject(
          new Error(
            `agent-browser ${args.join(' ')} failed: code=${code} signal=${signal}\n${out}${err}`,
          ),
        );
      }
    });
  });
}

function startMcpServer() {
  child = spawn('cargo', cargoArgs(['--session', session, 'mcp', 'serve']), {
    cwd: rootDir,
    env,
    stdio: ['pipe', 'pipe', 'pipe'],
  });

  child.stdout.setEncoding('utf8');
  child.stdout.on('data', (chunk) => {
    stdout += chunk;
    let newline = stdout.indexOf('\n');
    while (newline >= 0) {
      const line = stdout.slice(0, newline).trim();
      stdout = stdout.slice(newline + 1);
      if (line) handleLine(line);
      newline = stdout.indexOf('\n');
    }
  });

  child.stderr.setEncoding('utf8');
  child.stderr.on('data', (chunk) => {
    stderr += chunk;
  });

  child.on('error', (err) => {
    fail(`Failed to spawn MCP server: ${err.message}`);
  });

  child.on('exit', (code, signal) => {
    if (pending.size > 0) {
      fail(`MCP server exited before all responses arrived: code=${code} signal=${signal}`);
    }
  });
}

function send(method, params) {
  const id = nextId++;
  const request = { jsonrpc: '2.0', id, method };
  if (params !== undefined) request.params = params;
  const promise = new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject, method });
  });
  child.stdin.write(`${JSON.stringify(request)}\n`);
  return promise;
}

function notify(method, params) {
  const notification = { jsonrpc: '2.0', method };
  if (params !== undefined) notification.params = params;
  child.stdin.write(`${JSON.stringify(notification)}\n`);
}

function handleLine(line) {
  let message;
  try {
    message = JSON.parse(line);
  } catch (err) {
    fail(`MCP server emitted non-JSON stdout line: ${line}\n${err.message}`);
    return;
  }

  const pendingRequest = pending.get(message.id);
  if (!pendingRequest) {
    fail(`Received unexpected MCP response id: ${message.id}`);
    return;
  }

  pending.delete(message.id);
  if (message.error) {
    pendingRequest.reject(
      new Error(`${pendingRequest.method} failed: ${JSON.stringify(message.error)}`),
    );
    return;
  }
  pendingRequest.resolve(message.result);
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function parseJsonOutput(output, label) {
  const text = output.trim();
  try {
    return JSON.parse(text);
  } catch (err) {
    throw new Error(`Failed to parse ${label} JSON output: ${err.message}\n${output}`);
  }
}

function parseToolPayload(result) {
  const text = result.content?.[0]?.text;
  assert(typeof text === 'string', 'MCP tool response missing text content');
  return JSON.parse(text);
}

async function cleanup() {
  clearTimeout(timeout);
  if (child) {
    child.stdin.end();
    child.kill('SIGTERM');
  }
  try {
    await runCli(['--json', '--session', session, 'close']);
  } catch {
    // The smoke may fail before the daemon starts; cleanup must stay best-effort.
  }
  rmSync(tempHome, { recursive: true, force: true });
}

async function fail(message) {
  for (const { reject } of pending.values()) reject(new Error(message));
  pending.clear();
  await cleanup();
  console.error(message);
  if (stderr.trim()) console.error(stderr.trim());
  process.exit(1);
}

try {
  const pageHtml = [
    '<!doctype html>',
    '<html>',
    '<head><title>MCP Live Smoke</title></head>',
    '<body>',
    '<main id="main">',
    '<h1>MCP Live Smoke</h1>',
    '<button id="ready">Ready</button>',
    '<a href="https://example.com/">Example</a>',
    '</main>',
    '</body>',
    '</html>',
  ].join('');
  const pageUrl = `data:text/html;charset=utf-8,${encodeURIComponent(pageHtml)}`;

  const openResult = await runCli([
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

  await cleanup();
  console.log('MCP live smoke passed');
} catch (err) {
  await fail(err.message);
}
