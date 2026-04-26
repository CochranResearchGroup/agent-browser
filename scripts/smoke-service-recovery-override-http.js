#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  assertRecoveryBudgetBlockedEvents,
  assertRecoveryOverrideEvents,
  assertRecoveryTraceEvents,
  closeSession,
  createSmokeContext,
  httpJson,
  httpJsonResult,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-sroh-', sessionPrefix: 'sroh' });
context.env.AGENT_BROWSER_SERVICE_RECOVERY_RETRY_BUDGET = '1';
context.env.AGENT_BROWSER_SERVICE_RECOVERY_BASE_BACKOFF_MS = '1';
context.env.AGENT_BROWSER_SERVICE_RECOVERY_MAX_BACKOFF_MS = '1';

const { session } = context;
const serviceName = 'RecoveryOverrideHttpSmoke';
const agentName = 'smoke-agent';
const taskName = 'resumeAfterFaultedHttpSmoke';
const traceFields = { serviceName, agentName, taskName };
const retryActor = 'service-recovery-override-http-smoke';

const timeout = setTimeout(() => {
  fail('Timed out waiting for service recovery override HTTP smoke to complete');
}, 120000);

async function cleanup() {
  clearTimeout(timeout);
  await closeSession(context);
  context.cleanupTempHome();
}

async function fail(message) {
  console.error(message);
  await cleanup();
  process.exit(1);
}

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

async function command(port, body) {
  const response = await httpJson(port, 'POST', '/api/command', { ...body, ...traceFields });
  assert(response.success === true, `HTTP command ${body.action} failed: ${JSON.stringify(response)}`);
  return response;
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

async function serviceTrace(port, { filtered = true } = {}) {
  const path = filtered
    ? `/api/service/trace?service-name=${encodeURIComponent(serviceName)}&agent-name=${encodeURIComponent(
        agentName,
      )}&task-name=${encodeURIComponent(taskName)}&limit=80`
    : '/api/service/trace?limit=80';
  const trace = await httpJson(
    port,
    'GET',
    path,
  );
  assert(trace.success === true, `HTTP service trace failed: ${JSON.stringify(trace)}`);
  assert(Array.isArray(trace.data?.events), 'HTTP service trace missing events array');
  return trace;
}

try {
  const initialUrl = dataUrl('Recovery Override HTTP Smoke', 'Recovery Override HTTP Smoke');
  const blockedUrl = dataUrl('Blocked Recovery Override HTTP Smoke', 'Blocked Recovery Override HTTP Smoke');
  const recoveredUrl = dataUrl('Recovered Recovery Override HTTP Smoke', 'Recovered Recovery Override HTTP Smoke');

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

  const streamStatusResult = await runCli(context, ['--json', '--session', session, 'stream', 'status']);
  let stream = parseJsonOutput(streamStatusResult.stdout, 'stream status');
  assert(
    stream.success === true,
    `stream status failed: ${streamStatusResult.stdout}${streamStatusResult.stderr}`,
  );
  if (!stream.data?.enabled) {
    const streamResult = await runCli(context, ['--json', '--session', session, 'stream', 'enable']);
    stream = parseJsonOutput(streamResult.stdout, 'stream enable');
    assert(stream.success === true, `stream enable failed: ${streamResult.stdout}${streamResult.stderr}`);
  }
  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream enable did not return a port: ${JSON.stringify(stream)}`);

  await command(port, {
    id: 'service-recovery-override-http-smoke-launch',
    action: 'launch',
    headless: true,
    args: ['--no-sandbox'],
  });

  const pidResponse = await command(port, {
    id: 'service-recovery-override-http-smoke-browser-pid',
    action: 'browser_pid',
  });
  const pid = pidResponse.data?.pid;
  assert(Number.isInteger(pid) && pid > 0, `browser_pid did not return a pid: ${JSON.stringify(pidResponse)}`);

  const browserId = `session:${session}`;
  appendPriorRecoveryAttempt(browserId);
  process.kill(pid, 'SIGKILL');

  const blocked = await httpJsonResult(port, 'POST', '/api/browser/navigate', {
    url: blockedUrl,
    ...traceFields,
  });
  assert(blocked.statusCode === 200, `HTTP browser navigate returned ${blocked.statusCode}`);
  assert(
    blocked.body?.success === false &&
      typeof blocked.body?.error === 'string' &&
      blocked.body.error.includes('retry budget exceeded'),
    `HTTP browser navigate did not report retry budget exhaustion: ${JSON.stringify(blocked)}`,
  );

  let trace = await serviceTrace(port);
  const blockedEvents = trace.data.events;
  assertRecoveryBudgetBlockedEvents(blockedEvents, { browserId, label: 'HTTP recovery override' });

  const retry = await httpJson(
    port,
    'POST',
    `/api/service/browsers/${browserId}/retry?by=${encodeURIComponent(
      retryActor,
    )}&note=${encodeURIComponent('HTTP smoke retry after intentional budget exhaustion')}`,
  );
  assert(retry.success === true, `HTTP retry failed: ${JSON.stringify(retry)}`);
  assert(retry.data?.retryEnabled === true, `HTTP retry did not enable recovery: ${JSON.stringify(retry)}`);
  assert(
    retry.data?.browser?.health === 'process_exited',
    `HTTP retry did not move browser back to process_exited: ${JSON.stringify(retry)}`,
  );

  trace = await serviceTrace(port, { filtered: false });
  const { overrideIndex } = assertRecoveryOverrideEvents(trace.data.events, {
    browserId,
    actor: retryActor,
    label: 'HTTP recovery override',
  });

  const recovered = await httpJson(port, 'POST', '/api/browser/navigate', {
    url: recoveredUrl,
    ...traceFields,
  });
  assert(
    recovered.success === true,
    `HTTP browser navigate did not recover after retry override: ${JSON.stringify(recovered)}`,
  );

  const title = await httpJson(port, 'GET', '/api/browser/title');
  assert(title.success === true, `HTTP browser title failed after retry override: ${JSON.stringify(title)}`);
  assert(
    title.data?.title === 'Recovered Recovery Override HTTP Smoke',
    `Recovered browser title was ${title.data?.title}`,
  );

  trace = await serviceTrace(port);
  const finalEvents = trace.data.events;
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
    label: 'HTTP recovery after override',
  });
  assert(
    trace.data.counts?.events === finalEvents.length,
    'HTTP service trace event count does not match returned events',
  );

  await cleanup();
  console.log('Service recovery override HTTP smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
