#!/usr/bin/env node

import {
  assert,
  assertRecoveryTraceEvents,
  assertServiceTracePayload,
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
  assertServiceTracePayload(trace, 'HTTP service trace');
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
  assertRecoveryTraceEvents(events, { browserId, label: 'HTTP recovery' });

  await cleanup();
  console.log('Service recovery HTTP trace smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
