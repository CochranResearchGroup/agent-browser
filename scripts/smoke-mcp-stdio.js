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
  const getUrlTool = tools.tools?.find((tool) => tool.name === 'browser_get_url');
  assert(getUrlTool, 'MCP browser_get_url tool missing');
  assert(
    getUrlTool.inputSchema?.properties?.serviceName,
    'MCP browser_get_url missing serviceName trace field',
  );
  assert(
    getUrlTool.inputSchema?.properties?.agentName,
    'MCP browser_get_url missing agentName trace field',
  );
  assert(
    getUrlTool.inputSchema?.properties?.taskName,
    'MCP browser_get_url missing taskName trace field',
  );
  const getTitleTool = tools.tools?.find((tool) => tool.name === 'browser_get_title');
  assert(getTitleTool, 'MCP browser_get_title tool missing');
  assert(
    getTitleTool.inputSchema?.properties?.serviceName,
    'MCP browser_get_title missing serviceName trace field',
  );
  assert(
    getTitleTool.inputSchema?.properties?.agentName,
    'MCP browser_get_title missing agentName trace field',
  );
  assert(
    getTitleTool.inputSchema?.properties?.taskName,
    'MCP browser_get_title missing taskName trace field',
  );
  const tabsTool = tools.tools?.find((tool) => tool.name === 'browser_tabs');
  assert(tabsTool, 'MCP browser_tabs tool missing');
  assert(
    tabsTool.inputSchema?.properties?.verbose,
    'MCP browser_tabs missing verbose option',
  );
  assert(
    tabsTool.inputSchema?.properties?.serviceName,
    'MCP browser_tabs missing serviceName trace field',
  );
  assert(
    tabsTool.inputSchema?.properties?.agentName,
    'MCP browser_tabs missing agentName trace field',
  );
  assert(
    tabsTool.inputSchema?.properties?.taskName,
    'MCP browser_tabs missing taskName trace field',
  );
  const screenshotTool = tools.tools?.find((tool) => tool.name === 'browser_screenshot');
  assert(screenshotTool, 'MCP browser_screenshot tool missing');
  assert(
    screenshotTool.inputSchema?.properties?.selector,
    'MCP browser_screenshot missing selector option',
  );
  assert(
    screenshotTool.inputSchema?.properties?.format,
    'MCP browser_screenshot missing format option',
  );
  assert(
    screenshotTool.inputSchema?.properties?.serviceName,
    'MCP browser_screenshot missing serviceName trace field',
  );
  assert(
    screenshotTool.inputSchema?.properties?.agentName,
    'MCP browser_screenshot missing agentName trace field',
  );
  assert(
    screenshotTool.inputSchema?.properties?.taskName,
    'MCP browser_screenshot missing taskName trace field',
  );
  const clickTool = tools.tools?.find((tool) => tool.name === 'browser_click');
  assert(clickTool, 'MCP browser_click tool missing');
  assert(
    clickTool.inputSchema?.required?.includes('selector'),
    'MCP browser_click missing selector requirement',
  );
  assert(
    clickTool.inputSchema?.properties?.newTab,
    'MCP browser_click missing newTab option',
  );
  assert(
    clickTool.inputSchema?.properties?.serviceName,
    'MCP browser_click missing serviceName trace field',
  );
  assert(
    clickTool.inputSchema?.properties?.agentName,
    'MCP browser_click missing agentName trace field',
  );
  assert(
    clickTool.inputSchema?.properties?.taskName,
    'MCP browser_click missing taskName trace field',
  );
  const fillTool = tools.tools?.find((tool) => tool.name === 'browser_fill');
  assert(fillTool, 'MCP browser_fill tool missing');
  assert(
    fillTool.inputSchema?.required?.includes('selector'),
    'MCP browser_fill missing selector requirement',
  );
  assert(
    fillTool.inputSchema?.required?.includes('value'),
    'MCP browser_fill missing value requirement',
  );
  assert(
    fillTool.inputSchema?.properties?.value,
    'MCP browser_fill missing value property',
  );
  assert(
    fillTool.inputSchema?.properties?.serviceName,
    'MCP browser_fill missing serviceName trace field',
  );
  assert(
    fillTool.inputSchema?.properties?.agentName,
    'MCP browser_fill missing agentName trace field',
  );
  assert(
    fillTool.inputSchema?.properties?.taskName,
    'MCP browser_fill missing taskName trace field',
  );
  const waitTool = tools.tools?.find((tool) => tool.name === 'browser_wait');
  assert(waitTool, 'MCP browser_wait tool missing');
  assert(
    waitTool.inputSchema?.properties?.selector,
    'MCP browser_wait missing selector option',
  );
  assert(
    waitTool.inputSchema?.properties?.text,
    'MCP browser_wait missing text option',
  );
  assert(
    waitTool.inputSchema?.properties?.loadState,
    'MCP browser_wait missing loadState option',
  );
  assert(
    waitTool.inputSchema?.properties?.timeoutMs,
    'MCP browser_wait missing timeoutMs option',
  );
  assert(
    waitTool.inputSchema?.properties?.serviceName,
    'MCP browser_wait missing serviceName trace field',
  );
  assert(
    waitTool.inputSchema?.properties?.agentName,
    'MCP browser_wait missing agentName trace field',
  );
  assert(
    waitTool.inputSchema?.properties?.taskName,
    'MCP browser_wait missing taskName trace field',
  );
  const typeTool = tools.tools?.find((tool) => tool.name === 'browser_type');
  assert(typeTool, 'MCP browser_type tool missing');
  assert(
    typeTool.inputSchema?.required?.includes('selector'),
    'MCP browser_type missing selector requirement',
  );
  assert(
    typeTool.inputSchema?.required?.includes('text'),
    'MCP browser_type missing text requirement',
  );
  assert(
    typeTool.inputSchema?.properties?.clear,
    'MCP browser_type missing clear option',
  );
  assert(
    typeTool.inputSchema?.properties?.delayMs,
    'MCP browser_type missing delayMs option',
  );
  assert(
    typeTool.inputSchema?.properties?.serviceName,
    'MCP browser_type missing serviceName trace field',
  );
  assert(
    typeTool.inputSchema?.properties?.agentName,
    'MCP browser_type missing agentName trace field',
  );
  assert(
    typeTool.inputSchema?.properties?.taskName,
    'MCP browser_type missing taskName trace field',
  );
  const pressTool = tools.tools?.find((tool) => tool.name === 'browser_press');
  assert(pressTool, 'MCP browser_press tool missing');
  assert(
    pressTool.inputSchema?.required?.includes('key'),
    'MCP browser_press missing key requirement',
  );
  assert(
    pressTool.inputSchema?.properties?.key,
    'MCP browser_press missing key property',
  );
  assert(
    pressTool.inputSchema?.properties?.serviceName,
    'MCP browser_press missing serviceName trace field',
  );
  assert(
    pressTool.inputSchema?.properties?.agentName,
    'MCP browser_press missing agentName trace field',
  );
  assert(
    pressTool.inputSchema?.properties?.taskName,
    'MCP browser_press missing taskName trace field',
  );
  const hoverTool = tools.tools?.find((tool) => tool.name === 'browser_hover');
  assert(hoverTool, 'MCP browser_hover tool missing');
  assert(
    hoverTool.inputSchema?.required?.includes('selector'),
    'MCP browser_hover missing selector requirement',
  );
  assert(
    hoverTool.inputSchema?.properties?.selector,
    'MCP browser_hover missing selector property',
  );
  assert(
    hoverTool.inputSchema?.properties?.serviceName,
    'MCP browser_hover missing serviceName trace field',
  );
  assert(
    hoverTool.inputSchema?.properties?.agentName,
    'MCP browser_hover missing agentName trace field',
  );
  assert(
    hoverTool.inputSchema?.properties?.taskName,
    'MCP browser_hover missing taskName trace field',
  );
  const selectTool = tools.tools?.find((tool) => tool.name === 'browser_select');
  assert(selectTool, 'MCP browser_select tool missing');
  assert(
    selectTool.inputSchema?.required?.includes('selector'),
    'MCP browser_select missing selector requirement',
  );
  assert(
    selectTool.inputSchema?.required?.includes('values'),
    'MCP browser_select missing values requirement',
  );
  assert(
    selectTool.inputSchema?.properties?.values,
    'MCP browser_select missing values property',
  );
  assert(
    selectTool.inputSchema?.properties?.serviceName,
    'MCP browser_select missing serviceName trace field',
  );
  assert(
    selectTool.inputSchema?.properties?.agentName,
    'MCP browser_select missing agentName trace field',
  );
  assert(
    selectTool.inputSchema?.properties?.taskName,
    'MCP browser_select missing taskName trace field',
  );
  const checkTool = tools.tools?.find((tool) => tool.name === 'browser_check');
  assert(checkTool, 'MCP browser_check tool missing');
  assert(
    checkTool.inputSchema?.required?.includes('selector'),
    'MCP browser_check missing selector requirement',
  );
  assert(
    checkTool.inputSchema?.properties?.selector,
    'MCP browser_check missing selector property',
  );
  assert(
    checkTool.inputSchema?.properties?.serviceName,
    'MCP browser_check missing serviceName trace field',
  );
  assert(
    checkTool.inputSchema?.properties?.agentName,
    'MCP browser_check missing agentName trace field',
  );
  assert(
    checkTool.inputSchema?.properties?.taskName,
    'MCP browser_check missing taskName trace field',
  );
  const uncheckTool = tools.tools?.find((tool) => tool.name === 'browser_uncheck');
  assert(uncheckTool, 'MCP browser_uncheck tool missing');
  assert(
    uncheckTool.inputSchema?.required?.includes('selector'),
    'MCP browser_uncheck missing selector requirement',
  );
  assert(
    uncheckTool.inputSchema?.properties?.selector,
    'MCP browser_uncheck missing selector property',
  );
  assert(
    uncheckTool.inputSchema?.properties?.serviceName,
    'MCP browser_uncheck missing serviceName trace field',
  );
  assert(
    uncheckTool.inputSchema?.properties?.agentName,
    'MCP browser_uncheck missing agentName trace field',
  );
  assert(
    uncheckTool.inputSchema?.properties?.taskName,
    'MCP browser_uncheck missing taskName trace field',
  );
  const scrollTool = tools.tools?.find((tool) => tool.name === 'browser_scroll');
  assert(scrollTool, 'MCP browser_scroll tool missing');
  assert(
    scrollTool.inputSchema?.properties?.direction,
    'MCP browser_scroll missing direction property',
  );
  assert(scrollTool.inputSchema?.properties?.amount, 'MCP browser_scroll missing amount property');
  assert(scrollTool.inputSchema?.properties?.deltaX, 'MCP browser_scroll missing deltaX property');
  assert(scrollTool.inputSchema?.properties?.deltaY, 'MCP browser_scroll missing deltaY property');
  assert(
    scrollTool.inputSchema?.properties?.selector,
    'MCP browser_scroll missing selector property',
  );
  assert(
    scrollTool.inputSchema?.properties?.serviceName,
    'MCP browser_scroll missing serviceName trace field',
  );
  assert(
    scrollTool.inputSchema?.properties?.agentName,
    'MCP browser_scroll missing agentName trace field',
  );
  assert(
    scrollTool.inputSchema?.properties?.taskName,
    'MCP browser_scroll missing taskName trace field',
  );
  const scrollIntoViewTool = tools.tools?.find(
    (tool) => tool.name === 'browser_scroll_into_view',
  );
  assert(scrollIntoViewTool, 'MCP browser_scroll_into_view tool missing');
  assert(
    scrollIntoViewTool.inputSchema?.required?.includes('selector'),
    'MCP browser_scroll_into_view missing selector requirement',
  );
  assert(
    scrollIntoViewTool.inputSchema?.properties?.selector,
    'MCP browser_scroll_into_view missing selector property',
  );
  assert(
    scrollIntoViewTool.inputSchema?.properties?.serviceName,
    'MCP browser_scroll_into_view missing serviceName trace field',
  );
  assert(
    scrollIntoViewTool.inputSchema?.properties?.agentName,
    'MCP browser_scroll_into_view missing agentName trace field',
  );
  assert(
    scrollIntoViewTool.inputSchema?.properties?.taskName,
    'MCP browser_scroll_into_view missing taskName trace field',
  );
  const focusTool = tools.tools?.find((tool) => tool.name === 'browser_focus');
  assert(focusTool, 'MCP browser_focus tool missing');
  assert(
    focusTool.inputSchema?.required?.includes('selector'),
    'MCP browser_focus missing selector requirement',
  );
  assert(
    focusTool.inputSchema?.properties?.selector,
    'MCP browser_focus missing selector property',
  );
  assert(
    focusTool.inputSchema?.properties?.serviceName,
    'MCP browser_focus missing serviceName trace field',
  );
  assert(
    focusTool.inputSchema?.properties?.agentName,
    'MCP browser_focus missing agentName trace field',
  );
  assert(
    focusTool.inputSchema?.properties?.taskName,
    'MCP browser_focus missing taskName trace field',
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
