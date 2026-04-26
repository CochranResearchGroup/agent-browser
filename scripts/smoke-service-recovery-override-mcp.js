#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  assertRecoveryBudgetBlockedEvents,
  assertRecoveryOverrideEvents,
  assertRecoveryTraceEvents,
  closeSession,
  createMcpStdioClient,
  createSmokeContext,
  parseJsonOutput,
  runCli,
  sendRawCommand,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-srom-',
  sessionPrefix: 'srom',
});
context.env.AGENT_BROWSER_SERVICE_RECOVERY_RETRY_BUDGET = '1';
context.env.AGENT_BROWSER_SERVICE_RECOVERY_BASE_BACKOFF_MS = '1';
context.env.AGENT_BROWSER_SERVICE_RECOVERY_MAX_BACKOFF_MS = '1';

const { session } = context;
const serviceName = 'RecoveryOverrideMcpSmoke';
const agentName = 'smoke-agent';
const taskName = 'resumeAfterFaultedMcpSmoke';
const traceFields = { serviceName, agentName, taskName };
const retryActor = 'service-recovery-override-mcp-smoke';

let mcp;
const timeout = setTimeout(() => {
  fail('Timed out waiting for service recovery override MCP smoke to complete');
}, 120000);

function dataUrl(title, heading) {
  const html = [
    '<!doctype html>',
    '<html>',
    `<head><title>${title}</title></head>`,
    `<body><h1 id="ready">${heading}</h1></body>`,
    '</html>',
  ].join('');
  return `data:text/html;charset=utf-8,${encodeURIComponent(html)}`;
}

function parseToolPayload(result) {
  const text = result.content?.[0]?.text;
  assert(typeof text === 'string', 'MCP tool response missing text content');
  return JSON.parse(text);
}

function send(method, params) {
  return mcp.send(method, params);
}

function notify(method, params) {
  mcp.notify(method, params);
}

async function cleanup() {
  clearTimeout(timeout);
  if (mcp) mcp.close();
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

async function fail(message) {
  if (mcp) mcp.rejectPending(message);
  console.error(message);
  const stderr = mcp?.stderr() ?? '';
  if (stderr.trim()) console.error(stderr.trim());
  await cleanup();
  process.exit(1);
}

function serviceStatePath() {
  return join(context.agentHome, 'service', 'state.json');
}

function appendPriorRecoveryAttempt(browserId) {
  const path = serviceStatePath();
  const state = JSON.parse(readFileSync(path, 'utf8'));
  state.events.push({
    id: `event-smoke-prior-recovery-${Date.now()}`,
    timestamp: new Date().toISOString(),
    kind: 'browser_recovery_started',
    message: `Browser ${browserId} recovery started`,
    browserId,
    sessionId: session,
    serviceName,
    agentName,
    taskName,
    currentHealth: 'process_exited',
    details: {
      reasonKind: 'process_exited',
      reason: 'Synthetic prior recovery attempt for blocked recovery override smoke',
      attempt: 1,
      retryBudget: 1,
      retryBudgetExceeded: false,
      nextRetryDelayMs: 1,
    },
  });
  writeFileSync(path, `${JSON.stringify(state, null, 2)}\n`);
}

async function serviceTrace({ filtered = true } = {}) {
  const result = await send('tools/call', {
    name: 'service_trace',
    arguments: filtered
      ? {
          limit: 80,
          ...traceFields,
        }
      : {
          limit: 80,
        },
  });
  const payload = parseToolPayload(result);
  assert(payload.success === true, `MCP service_trace failed: ${JSON.stringify(payload)}`);
  assert(payload.tool === 'service_trace', 'service_trace payload tool mismatch');
  assert(Array.isArray(payload.data?.events), 'MCP service_trace missing events array');
  return payload;
}

try {
  const initialUrl = dataUrl('Recovery Override MCP Smoke', 'Recovery Override MCP Smoke');
  const blockedUrl = dataUrl('Blocked Recovery Override MCP Smoke', 'Blocked Recovery Override MCP Smoke');
  const recoveredUrl = dataUrl('Recovered Recovery Override MCP Smoke', 'Recovered Recovery Override MCP Smoke');

  const openResult = await runCli(context, [
    '--json',
    '--session',
    session,
    '--args',
    '--no-sandbox',
    'open',
    initialUrl,
  ]);
  const opened = parseJsonOutput(openResult.stdout, 'open');
  assert(opened.success === true, `Open command failed: ${openResult.stdout}${openResult.stderr}`);

  const launch = await sendRawCommand(context, {
    id: 'service-recovery-override-mcp-smoke-launch',
    action: 'launch',
    headless: true,
    args: ['--no-sandbox'],
    ...traceFields,
  });
  assert(launch.success === true, `raw launch failed: ${JSON.stringify(launch)}`);

  mcp = createMcpStdioClient({
    context,
    args: ['--session', session, 'mcp', 'serve'],
    onFatal: fail,
  });
  const initialize = await send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'agent-browser-service-recovery-override-mcp-smoke', version: '0' },
  });
  assert(initialize.capabilities?.tools, 'MCP tools capability missing');
  notify('notifications/initialized');

  const pidResponse = await sendRawCommand(context, {
    id: 'service-recovery-override-mcp-smoke-browser-pid',
    action: 'browser_pid',
    ...traceFields,
  });
  assert(pidResponse.success === true, `browser_pid failed: ${JSON.stringify(pidResponse)}`);
  const pid = pidResponse.data?.pid;
  assert(Number.isInteger(pid) && pid > 0, `browser_pid did not return a pid: ${JSON.stringify(pidResponse)}`);

  const browserId = `session:${session}`;
  appendPriorRecoveryAttempt(browserId);
  process.kill(pid, 'SIGKILL');

  const blockedResult = await send('tools/call', {
    name: 'browser_navigate',
    arguments: {
      url: blockedUrl,
      waitUntil: 'load',
      ...traceFields,
    },
  });
  const blockedPayload = parseToolPayload(blockedResult);
  assert(
    blockedPayload.success === false &&
      typeof blockedPayload.error === 'string' &&
      blockedPayload.error.includes('retry budget exceeded'),
    `MCP browser_navigate did not report retry budget exhaustion: ${JSON.stringify(blockedPayload)}`,
  );

  let tracePayload = await serviceTrace();
  assertRecoveryBudgetBlockedEvents(tracePayload.data.events, {
    browserId,
    label: 'MCP recovery override',
  });

  const retryResult = await send('tools/call', {
    name: 'service_browser_retry',
    arguments: {
      browserId,
      by: retryActor,
      note: 'MCP smoke retry after intentional budget exhaustion',
    },
  });
  const retryPayload = parseToolPayload(retryResult);
  assert(retryPayload.success === true, `MCP service_browser_retry failed: ${JSON.stringify(retryPayload)}`);
  assert(
    retryPayload.data?.retryEnabled === true,
    `MCP service_browser_retry did not enable recovery: ${JSON.stringify(retryPayload)}`,
  );
  assert(
    retryPayload.data?.browser?.health === 'process_exited',
    `MCP service_browser_retry did not move browser back to process_exited: ${JSON.stringify(retryPayload)}`,
  );

  tracePayload = await serviceTrace({ filtered: false });
  const { overrideIndex } = assertRecoveryOverrideEvents(tracePayload.data.events, {
    browserId,
    actor: retryActor,
    label: 'MCP recovery override',
  });

  const recoveredResult = await send('tools/call', {
    name: 'browser_navigate',
    arguments: {
      url: recoveredUrl,
      waitUntil: 'load',
      ...traceFields,
    },
  });
  const recoveredPayload = parseToolPayload(recoveredResult);
  assert(
    recoveredPayload.success === true,
    `MCP browser_navigate did not recover after retry override: ${JSON.stringify(recoveredPayload)}`,
  );

  const titleResult = await send('tools/call', {
    name: 'browser_get_title',
    arguments: traceFields,
  });
  const titlePayload = parseToolPayload(titleResult);
  assert(titlePayload.success === true, `MCP browser_get_title failed: ${JSON.stringify(titlePayload)}`);
  assert(
    titlePayload.data?.title === 'Recovered Recovery Override MCP Smoke',
    `Recovered browser title was ${titlePayload.data?.title}`,
  );

  tracePayload = await serviceTrace();
  const finalEvents = tracePayload.data.events;
  const retryHealthIndex = finalEvents.findIndex(
    (event) =>
      event.kind === 'browser_health_changed' &&
      event.browserId === browserId &&
      event.previousHealth === 'faulted' &&
      event.currentHealth === 'process_exited',
  );
  assert(
    retryHealthIndex >= 0,
    `Final filtered trace lost the retry health event after override index ${overrideIndex}: ${JSON.stringify(finalEvents)}`,
  );
  assertRecoveryTraceEvents(finalEvents.slice(retryHealthIndex), {
    browserId,
    label: 'MCP recovery after override',
  });
  assert(
    tracePayload.data.counts?.events === finalEvents.length,
    'MCP service_trace event count does not match returned events',
  );

  await cleanup();
  console.log('Service recovery override MCP smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
