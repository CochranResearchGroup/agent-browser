#!/usr/bin/env node

import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-srh-', sessionPrefix: 'srh' });
const { session } = context;
const serviceName = 'RecoveryTraceHttpSmoke';
const agentName = 'smoke-agent';
const taskName = 'recoverAfterCrashHttpSmoke';
const traceFields = { serviceName, agentName, taskName };
const staleHealthValues = new Set(['process_exited', 'cdp_disconnected']);

const timeout = setTimeout(() => {
  fail('Timed out waiting for service recovery HTTP smoke to complete');
}, 90000);

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

function eventIndex(events, predicate, label) {
  const index = events.findIndex(predicate);
  assert(index >= 0, `${label} missing from trace events: ${JSON.stringify(events)}`);
  return index;
}

try {
  const initialUrl = dataUrl('Recovery Trace HTTP Smoke', 'Recovery Trace HTTP Smoke');
  const recoveredUrl = dataUrl('Recovered Recovery Trace HTTP Smoke', 'Recovered Recovery Trace HTTP Smoke');

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
    id: 'service-recovery-http-smoke-launch',
    action: 'launch',
    headless: true,
    args: ['--no-sandbox'],
  });

  const pidResponse = await command(port, {
    id: 'service-recovery-http-smoke-browser-pid',
    action: 'browser_pid',
  });
  const pid = pidResponse.data?.pid;
  assert(Number.isInteger(pid) && pid > 0, `browser_pid did not return a pid: ${JSON.stringify(pidResponse)}`);

  process.kill(pid, 'SIGKILL');

  const recovered = await httpJson(port, 'POST', '/api/browser/navigate', {
    url: recoveredUrl,
    ...traceFields,
  });
  assert(
    recovered.success === true,
    `HTTP browser navigate did not recover after killed Chrome: ${JSON.stringify(recovered)}`,
  );

  const title = await httpJson(port, 'GET', '/api/browser/title');
  assert(title.success === true, `HTTP browser title failed after recovery: ${JSON.stringify(title)}`);
  assert(
    title.data?.title === 'Recovered Recovery Trace HTTP Smoke',
    `Recovered browser title was ${title.data?.title}`,
  );

  const trace = await httpJson(
    port,
    'GET',
    `/api/service/trace?service-name=${encodeURIComponent(serviceName)}&agent-name=${encodeURIComponent(
      agentName,
    )}&task-name=${encodeURIComponent(taskName)}&limit=50`,
  );
  assert(trace.success === true, `HTTP service trace failed: ${JSON.stringify(trace)}`);
  assert(Array.isArray(trace.data?.events), 'HTTP service trace missing events array');
  assert(
    trace.data?.filters?.serviceName === serviceName,
    `Trace service filter was ${trace.data?.filters?.serviceName}`,
  );
  assert(
    trace.data?.filters?.agentName === agentName,
    `Trace agent filter was ${trace.data?.filters?.agentName}`,
  );
  assert(
    trace.data?.filters?.taskName === taskName,
    `Trace task filter was ${trace.data?.filters?.taskName}`,
  );

  const events = trace.data.events;
  const browserId = `session:${session}`;
  const staleIndex = eventIndex(
    events,
    (event) =>
      event.kind === 'browser_health_changed' &&
      event.browserId === browserId &&
      staleHealthValues.has(event.currentHealth),
    'stale browser health event',
  );
  const staleHealth = events[staleIndex].currentHealth;
  const recoveryIndex = eventIndex(
    events,
    (event) =>
      event.kind === 'browser_recovery_started' &&
      event.browserId === browserId &&
      event.currentHealth === staleHealth,
    'browser recovery started event',
  );
  const readyIndex = eventIndex(
    events,
    (event) =>
      event.kind === 'browser_health_changed' &&
      event.browserId === browserId &&
      event.currentHealth === 'ready',
    'ready browser health event',
  );

  assert(
    staleIndex < recoveryIndex && recoveryIndex < readyIndex,
    `Recovery events were not ordered stale -> recovery -> ready: ${JSON.stringify(events)}`,
  );
  assert(
    typeof events[recoveryIndex].details?.reason === 'string' &&
      events[recoveryIndex].details.reason.length > 0,
    `Recovery event did not include crash reason: ${JSON.stringify(events[recoveryIndex])}`,
  );
  assert(
    trace.data.counts?.events === events.length,
    'HTTP service trace event count does not match returned events',
  );

  await cleanup();
  console.log('Service recovery HTTP trace smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
