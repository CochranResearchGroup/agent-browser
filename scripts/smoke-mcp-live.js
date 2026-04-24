#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs';
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
const screenshotDir = join(tempHome, 'screenshots');
const serviceName = 'McpLiveSmoke';
const agentName = 'smoke-agent';
const taskName = 'browserSnapshotSmoke';

mkdirSync(socketDir, { recursive: true });
mkdirSync(profileDir, { recursive: true });
mkdirSync(screenshotDir, { recursive: true });

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
    '<button id="ready" onclick="document.getElementById(\'status\').textContent = \'Clicked\'">Ready</button>',
    '<p id="status">Not clicked</p>',
    '<label for="name">Name</label>',
    '<input id="name" name="name" value="">',
    '<button id="copy-name" onclick="document.getElementById(\'name-output\').textContent = document.getElementById(\'name\').value">Copy name</button>',
    '<p id="name-output"></p>',
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
      clickedSnapshotPayload.data.snapshot.includes('Ada Lovelace'),
    'browser_fill did not mutate the page state visible to a follow-up snapshot',
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
  const fillJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'fill' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(fillJob, 'Retained service job with browser_fill caller context was not found');
  assert(fillJob.state === 'succeeded', `Fill service job state was ${fillJob.state}`);

  await cleanup();
  console.log('MCP live smoke passed');
} catch (err) {
  await fail(err.message);
}
