#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const rootDir = new URL('..', import.meta.url).pathname;
const tempHome = mkdtempSync(join(tmpdir(), 'agent-browser-mcp-smoke-'));
const realHome = process.env.HOME;
const cargoHome = process.env.CARGO_HOME || (realHome ? join(realHome, '.cargo') : undefined);
const rustupHome = process.env.RUSTUP_HOME || (realHome ? join(realHome, '.rustup') : undefined);
const child = spawn(
  'cargo',
  ['run', '--quiet', '--manifest-path', 'cli/Cargo.toml', '--', 'mcp', 'serve'],
  {
    cwd: rootDir,
    env: {
      ...process.env,
      HOME: tempHome,
      AGENT_BROWSER_HOME: join(tempHome, '.agent-browser'),
      ...(cargoHome ? { CARGO_HOME: cargoHome } : {}),
      ...(rustupHome ? { RUSTUP_HOME: rustupHome } : {}),
    },
    stdio: ['pipe', 'pipe', 'pipe'],
  },
);

let stdout = '';
let stderr = '';
let nextId = 1;
const pending = new Map();
const timeout = setTimeout(() => {
  fail('Timed out waiting for MCP stdio responses');
}, 45000);

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
  if (!condition) fail(message);
}

function fail(message) {
  clearTimeout(timeout);
  for (const { reject } of pending.values()) reject(new Error(message));
  pending.clear();
  child.kill('SIGTERM');
  rmSync(tempHome, { recursive: true, force: true });
  console.error(message);
  if (stderr.trim()) console.error(stderr.trim());
  process.exit(1);
}

try {
  const initialize = await send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'agent-browser-smoke', version: '0' },
  });
  assert(initialize.protocolVersion === '2025-06-18', 'Unexpected MCP protocol version');
  assert(initialize.capabilities?.resources, 'MCP resources capability missing');
  notify('notifications/initialized');

  const resources = await send('resources/list');
  assert(
    resources.resources?.some((resource) => resource.uri === 'agent-browser://incidents'),
    'MCP incidents resource missing',
  );
  assert(
    resources.resources?.some((resource) => resource.uri === 'agent-browser://profiles'),
    'MCP profiles resource missing',
  );
  assert(
    resources.resources?.some((resource) => resource.uri === 'agent-browser://sessions'),
    'MCP sessions resource missing',
  );
  assert(
    resources.resources?.some((resource) => resource.uri === 'agent-browser://browsers'),
    'MCP browsers resource missing',
  );
  assert(
    resources.resources?.some((resource) => resource.uri === 'agent-browser://tabs'),
    'MCP tabs resource missing',
  );
  assert(
    resources.resources?.some((resource) => resource.uri === 'agent-browser://site-policies'),
    'MCP site policies resource missing',
  );
  assert(
    resources.resources?.some((resource) => resource.uri === 'agent-browser://providers'),
    'MCP providers resource missing',
  );
  assert(
    resources.resources?.some((resource) => resource.uri === 'agent-browser://challenges'),
    'MCP challenges resource missing',
  );
  assert(
    resources.resources?.some((resource) => resource.uri === 'agent-browser://jobs'),
    'MCP jobs resource missing',
  );
  assert(
    resources.resources?.some((resource) => resource.uri === 'agent-browser://events'),
    'MCP events resource missing',
  );

  const templates = await send('resources/templates/list');
  assert(
    templates.resourceTemplates?.some(
      (resource) =>
        resource.uriTemplate === 'agent-browser://incidents/{incident_id}/activity',
    ),
    'MCP incident activity template missing',
  );

  const tools = await send('tools/list');
  const cancelTool = tools.tools?.find((tool) => tool.name === 'service_job_cancel');
  assert(cancelTool, 'MCP service_job_cancel tool missing');
  assert(
    cancelTool.inputSchema?.properties?.serviceName,
    'MCP service_job_cancel missing serviceName trace field',
  );
  assert(
    cancelTool.inputSchema?.properties?.agentName,
    'MCP service_job_cancel missing agentName trace field',
  );
  assert(
    cancelTool.inputSchema?.properties?.taskName,
    'MCP service_job_cancel missing taskName trace field',
  );
  const snapshotTool = tools.tools?.find((tool) => tool.name === 'browser_snapshot');
  assert(snapshotTool, 'MCP browser_snapshot tool missing');
  assert(
    snapshotTool.inputSchema?.properties?.interactive,
    'MCP browser_snapshot missing interactive option',
  );
  assert(
    snapshotTool.inputSchema?.properties?.serviceName,
    'MCP browser_snapshot missing serviceName trace field',
  );
  assert(
    snapshotTool.inputSchema?.properties?.agentName,
    'MCP browser_snapshot missing agentName trace field',
  );
  assert(
    snapshotTool.inputSchema?.properties?.taskName,
    'MCP browser_snapshot missing taskName trace field',
  );

  const incidents = await send('resources/read', { uri: 'agent-browser://incidents' });
  const incidentContent = incidents.contents?.[0];
  assert(incidentContent?.mimeType === 'application/json', 'MCP incident content MIME mismatch');
  assert(incidentContent?.uri === 'agent-browser://incidents', 'MCP incident content URI mismatch');
  const incidentPayload = JSON.parse(incidentContent.text);
  assert(Array.isArray(incidentPayload.incidents), 'MCP incident payload missing incidents array');
  assert(incidentPayload.count === 0, 'Fresh MCP smoke state should have zero incidents');

  const profiles = await send('resources/read', { uri: 'agent-browser://profiles' });
  const profilePayload = JSON.parse(profiles.contents?.[0]?.text || '{}');
  assert(Array.isArray(profilePayload.profiles), 'MCP profiles payload missing profiles array');
  assert(profilePayload.count === 0, 'Fresh MCP smoke state should have zero profiles');

  const sessions = await send('resources/read', { uri: 'agent-browser://sessions' });
  const sessionPayload = JSON.parse(sessions.contents?.[0]?.text || '{}');
  assert(Array.isArray(sessionPayload.sessions), 'MCP sessions payload missing sessions array');
  assert(sessionPayload.count === 0, 'Fresh MCP smoke state should have zero sessions');

  const browsers = await send('resources/read', { uri: 'agent-browser://browsers' });
  const browserPayload = JSON.parse(browsers.contents?.[0]?.text || '{}');
  assert(Array.isArray(browserPayload.browsers), 'MCP browsers payload missing browsers array');
  assert(browserPayload.count === 0, 'Fresh MCP smoke state should have zero browsers');

  const tabs = await send('resources/read', { uri: 'agent-browser://tabs' });
  const tabPayload = JSON.parse(tabs.contents?.[0]?.text || '{}');
  assert(Array.isArray(tabPayload.tabs), 'MCP tabs payload missing tabs array');
  assert(tabPayload.count === 0, 'Fresh MCP smoke state should have zero tabs');

  const sitePolicies = await send('resources/read', { uri: 'agent-browser://site-policies' });
  const sitePolicyPayload = JSON.parse(sitePolicies.contents?.[0]?.text || '{}');
  assert(
    Array.isArray(sitePolicyPayload.sitePolicies),
    'MCP site policies payload missing sitePolicies array',
  );
  assert(sitePolicyPayload.count === 0, 'Fresh MCP smoke state should have zero site policies');

  const providers = await send('resources/read', { uri: 'agent-browser://providers' });
  const providerPayload = JSON.parse(providers.contents?.[0]?.text || '{}');
  assert(Array.isArray(providerPayload.providers), 'MCP providers payload missing providers array');
  assert(providerPayload.count === 0, 'Fresh MCP smoke state should have zero providers');

  const challenges = await send('resources/read', { uri: 'agent-browser://challenges' });
  const challengePayload = JSON.parse(challenges.contents?.[0]?.text || '{}');
  assert(
    Array.isArray(challengePayload.challenges),
    'MCP challenges payload missing challenges array',
  );
  assert(challengePayload.count === 0, 'Fresh MCP smoke state should have zero challenges');

  const jobs = await send('resources/read', { uri: 'agent-browser://jobs' });
  const jobPayload = JSON.parse(jobs.contents?.[0]?.text || '{}');
  assert(Array.isArray(jobPayload.jobs), 'MCP jobs payload missing jobs array');
  assert(jobPayload.count === 0, 'Fresh MCP smoke state should have zero jobs');

  const events = await send('resources/read', { uri: 'agent-browser://events' });
  const eventPayload = JSON.parse(events.contents?.[0]?.text || '{}');
  assert(Array.isArray(eventPayload.events), 'MCP events payload missing events array');
  assert(eventPayload.count === 0, 'Fresh MCP smoke state should have zero events');

  clearTimeout(timeout);
  child.stdin.end();
  child.kill('SIGTERM');
  rmSync(tempHome, { recursive: true, force: true });
  console.log('MCP stdio smoke passed');
} catch (err) {
  fail(err.message);
}
