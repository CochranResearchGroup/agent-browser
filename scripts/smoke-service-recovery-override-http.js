#!/usr/bin/env node

import {
  retryServiceBrowser,
} from '../packages/client/src/service-observability.js';
import {
  appendPriorRecoveryAttempt,
  assert,
  assertRecoveryBudgetBlockedEvents,
  assertRecoveryAfterOverride,
  assertRecoveryOverrideEvents,
  assertServiceTracePayload,
  closeSession,
  configureRecoveryOverrideSmokeContext,
  createSmokeContext,
  httpJson,
  httpJsonResult,
  parseJsonOutput,
  recoveryOverrideSmokeUrls,
  runCli,
} from './smoke-utils.js';

const context = configureRecoveryOverrideSmokeContext(
  createSmokeContext({ prefix: 'ab-sroh-', sessionPrefix: 'sroh' }),
);

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

async function command(port, body) {
  const response = await httpJson(port, 'POST', '/api/command', { ...body, ...traceFields });
  assert(response.success === true, `HTTP command ${body.action} failed: ${JSON.stringify(response)}`);
  return response;
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
  return assertServiceTracePayload(trace, 'HTTP service trace');
}

try {
  const { blockedUrl, initialUrl, recoveredUrl } = recoveryOverrideSmokeUrls(
    'Recovery Override HTTP Smoke',
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
  appendPriorRecoveryAttempt(context, {
    agentName,
    browserId,
    serviceName,
    taskName,
  });
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

  const retry = await retryServiceBrowser({
    baseUrl: `http://127.0.0.1:${port}`,
    browserId,
    by: retryActor,
    note: 'HTTP smoke retry after intentional budget exhaustion',
    serviceName,
    agentName,
    taskName,
  });
  assert(retry.retryEnabled === true, `HTTP retry did not enable recovery: ${JSON.stringify(retry)}`);
  assert(
    retry.browser?.health === 'process_exited',
    `HTTP retry did not move browser back to process_exited: ${JSON.stringify(retry)}`,
  );

  trace = await serviceTrace(port);
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
  assertRecoveryAfterOverride(finalEvents, {
    browserId,
    label: 'HTTP recovery after override',
    overrideIndex,
  });

  await cleanup();
  console.log('Service recovery override HTTP smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
