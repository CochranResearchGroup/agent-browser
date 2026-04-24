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
}, 15000);

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

  const templates = await send('resources/templates/list');
  assert(
    templates.resourceTemplates?.some(
      (resource) =>
        resource.uriTemplate === 'agent-browser://incidents/{incident_id}/activity',
    ),
    'MCP incident activity template missing',
  );

  const incidents = await send('resources/read', { uri: 'agent-browser://incidents' });
  const incidentContent = incidents.contents?.[0];
  assert(incidentContent?.mimeType === 'application/json', 'MCP incident content MIME mismatch');
  assert(incidentContent?.uri === 'agent-browser://incidents', 'MCP incident content URI mismatch');
  const incidentPayload = JSON.parse(incidentContent.text);
  assert(Array.isArray(incidentPayload.incidents), 'MCP incident payload missing incidents array');
  assert(incidentPayload.count === 0, 'Fresh MCP smoke state should have zero incidents');

  clearTimeout(timeout);
  child.stdin.end();
  child.kill('SIGTERM');
  rmSync(tempHome, { recursive: true, force: true });
  console.log('MCP stdio smoke passed');
} catch (err) {
  fail(err.message);
}
