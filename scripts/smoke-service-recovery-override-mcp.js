#!/usr/bin/env node

import {
  appendPriorRecoveryAttempt,
  assert,
  assertRecoveryBudgetBlockedEvents,
  assertRecoveryAfterOverride,
  assertRecoveryOverrideEvents,
  assertServiceTracePayload,
  closeSession,
  configureRecoveryOverrideSmokeContext,
  createMcpStdioClient,
  createSmokeContext,
  parseMcpToolPayload,
  parseJsonOutput,
  recoveryOverrideSmokeUrls,
  runCli,
  sendRawCommand,
} from './smoke-utils.js';
import {
  assertServiceBrowserRetryResponseSchemaRecord,
  loadServiceRecordSchema,
} from './smoke-schema-utils.js';

const context = configureRecoveryOverrideSmokeContext(
  createSmokeContext({
    prefix: 'ab-srom-',
    sessionPrefix: 'srom',
  }),
);

const { session } = context;
const serviceName = 'RecoveryOverrideMcpSmoke';
const agentName = 'smoke-agent';
const taskName = 'resumeAfterFaultedMcpSmoke';
const traceFields = { serviceName, agentName, taskName };
const retryActor = 'service-recovery-override-mcp-smoke';
const browserRetryResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-browser-retry-response.v1.schema.json',
);

let mcp;
const timeout = setTimeout(() => {
  fail('Timed out waiting for service recovery override MCP smoke to complete');
}, 120000);

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
  const payload = parseMcpToolPayload(result);
  return assertServiceTracePayload(payload, 'MCP service_trace', { tool: 'service_trace' });
}

try {
  const { blockedUrl, initialUrl, recoveredUrl } = recoveryOverrideSmokeUrls(
    'Recovery Override MCP Smoke',
  );

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
  appendPriorRecoveryAttempt(context, {
    agentName,
    browserId,
    serviceName,
    taskName,
  });
  process.kill(pid, 'SIGKILL');

  const blockedResult = await send('tools/call', {
    name: 'browser_navigate',
    arguments: {
      url: blockedUrl,
      waitUntil: 'load',
      ...traceFields,
    },
  });
  const blockedPayload = parseMcpToolPayload(blockedResult);
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
      ...traceFields,
    },
  });
  const retryPayload = parseMcpToolPayload(retryResult);
  assert(retryPayload.success === true, `MCP service_browser_retry failed: ${JSON.stringify(retryPayload)}`);
  assertServiceBrowserRetryResponseSchemaRecord(
    retryPayload.data,
    browserRetryResponseSchema,
    'MCP service_browser_retry response',
  );
  assert(
    retryPayload.data?.retryEnabled === true,
    `MCP service_browser_retry did not enable recovery: ${JSON.stringify(retryPayload)}`,
  );
  assert(
    retryPayload.data?.browser?.health === 'process_exited',
    `MCP service_browser_retry did not move browser back to process_exited: ${JSON.stringify(retryPayload)}`,
  );

  tracePayload = await serviceTrace();
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
  const recoveredPayload = parseMcpToolPayload(recoveredResult);
  assert(
    recoveredPayload.success === true,
    `MCP browser_navigate did not recover after retry override: ${JSON.stringify(recoveredPayload)}`,
  );

  const titleResult = await send('tools/call', {
    name: 'browser_get_title',
    arguments: traceFields,
  });
  const titlePayload = parseMcpToolPayload(titleResult);
  assert(titlePayload.success === true, `MCP browser_get_title failed: ${JSON.stringify(titlePayload)}`);
  assert(
    titlePayload.data?.title === 'Recovered Recovery Override MCP Smoke',
    `Recovered browser title was ${titlePayload.data?.title}`,
  );

  tracePayload = await serviceTrace();
  const finalEvents = tracePayload.data.events;
  assertRecoveryAfterOverride(finalEvents, {
    browserId,
    label: 'MCP recovery after override',
    overrideIndex,
  });

  await cleanup();
  console.log('Service recovery override MCP smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
