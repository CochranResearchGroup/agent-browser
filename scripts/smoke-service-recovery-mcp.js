#!/usr/bin/env node

import {
  assert,
  assertRecoveryTraceEvents,
  assertServiceTracePayload,
  closeSession,
  createMcpStdioClient,
  createSmokeContext,
  parseJsonOutput,
  runCli,
  sendRawCommand,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-srm-',
  sessionPrefix: 'srm',
});
const { session } = context;
const serviceName = 'RecoveryTraceMcpSmoke';
const agentName = 'smoke-agent';
const taskName = 'recoverAfterCrashMcpSmoke';
const traceFields = { serviceName, agentName, taskName };

let mcp;
const timeout = setTimeout(() => {
  fail('Timed out waiting for service recovery MCP smoke to complete');
}, 90000);

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

try {
  const initialUrl = dataUrl('Recovery Trace MCP Smoke', 'Recovery Trace MCP Smoke');
  const recoveredUrl = dataUrl('Recovered Recovery Trace MCP Smoke', 'Recovered Recovery Trace MCP Smoke');

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
    id: 'service-recovery-mcp-smoke-launch',
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
    clientInfo: { name: 'agent-browser-service-recovery-mcp-smoke', version: '0' },
  });
  assert(initialize.capabilities?.tools, 'MCP tools capability missing');
  notify('notifications/initialized');

  const pidResponse = await sendRawCommand(context, {
    id: 'service-recovery-mcp-smoke-browser-pid',
    action: 'browser_pid',
    ...traceFields,
  });
  assert(pidResponse.success === true, `browser_pid failed: ${JSON.stringify(pidResponse)}`);
  const pid = pidResponse.data?.pid;
  assert(Number.isInteger(pid) && pid > 0, `browser_pid did not return a pid: ${JSON.stringify(pidResponse)}`);

  process.kill(pid, 'SIGKILL');

  const navigateResult = await send('tools/call', {
    name: 'browser_navigate',
    arguments: {
      url: recoveredUrl,
      waitUntil: 'load',
      ...traceFields,
    },
  });
  const navigatePayload = parseToolPayload(navigateResult);
  assert(
    navigatePayload.success === true,
    `MCP browser_navigate did not recover after killed Chrome: ${JSON.stringify(navigatePayload)}`,
  );
  assert(navigatePayload.tool === 'browser_navigate', 'browser_navigate payload tool mismatch');
  assert(navigatePayload.trace?.serviceName === serviceName, 'browser_navigate trace missing serviceName');
  assert(navigatePayload.trace?.agentName === agentName, 'browser_navigate trace missing agentName');
  assert(navigatePayload.trace?.taskName === taskName, 'browser_navigate trace missing taskName');

  const titleResult = await send('tools/call', {
    name: 'browser_get_title',
    arguments: traceFields,
  });
  const titlePayload = parseToolPayload(titleResult);
  assert(titlePayload.success === true, `MCP browser_get_title failed: ${JSON.stringify(titlePayload)}`);
  assert(
    titlePayload.data?.title === 'Recovered Recovery Trace MCP Smoke',
    `Recovered browser title was ${titlePayload.data?.title}`,
  );

  const traceResult = await send('tools/call', {
    name: 'service_trace',
    arguments: {
      limit: 50,
      ...traceFields,
    },
  });
  const tracePayload = parseToolPayload(traceResult);
  assertServiceTracePayload(tracePayload, 'MCP service_trace', { tool: 'service_trace' });
  assert(tracePayload.trace?.serviceName === serviceName, 'service_trace response trace missing serviceName');
  assert(tracePayload.trace?.agentName === agentName, 'service_trace response trace missing agentName');
  assert(tracePayload.trace?.taskName === taskName, 'service_trace response trace missing taskName');

  const events = tracePayload.data.events;
  const browserId = `session:${session}`;
  assertRecoveryTraceEvents(events, { browserId, label: 'MCP recovery' });
  assert(
    tracePayload.data.jobs.some(
      (job) =>
        job.action === 'navigate' &&
        job.serviceName === serviceName &&
        job.agentName === agentName &&
        job.taskName === taskName,
    ),
    `MCP service_trace did not retain the recovery navigate job: ${JSON.stringify(tracePayload.data.jobs)}`,
  );

  await cleanup();
  console.log('Service recovery MCP trace smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
