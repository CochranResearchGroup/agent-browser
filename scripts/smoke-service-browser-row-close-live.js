#!/usr/bin/env node

import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-browser-row-close-',
  sessionPrefix: 'browser-row-close',
});

context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'BrowserRowCloseSmoke';
const agentName = 'smoke-agent';
const taskName = 'httpBrowserCloseLive';
const browserId = `session:${session}`;

const timeout = setTimeout(() => {
  fail('Timed out waiting for service browser row close live smoke to complete');
}, 240000);

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

async function ensureStreamPort() {
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
  assert(Number.isInteger(port) && port > 0, `stream status did not return a port: ${JSON.stringify(stream)}`);
  return port;
}

try {
  const openResult = await runCli(
    context,
    [
      '--json',
      '--session',
      session,
      '--args',
      '--no-sandbox',
      'open',
      smokeDataUrl('Browser Row Close Smoke', 'Browser Row Close Smoke'),
    ],
    180000,
  );
  const opened = parseJsonOutput(openResult.stdout, 'open');
  assert(opened.success === true, `Open command failed: ${openResult.stdout}${openResult.stderr}`);

  const port = await ensureStreamPort();
  const contracts = await httpJson(port, 'GET', '/api/service/contracts');
  const actions = contracts.data?.contracts?.serviceRequest?.actions ?? [];
  assert(actions.includes('service_browser_close'), `contracts missing service_browser_close: ${JSON.stringify(actions)}`);

  const closeResponse = await httpJson(port, 'POST', '/api/service/request', {
    action: 'service_browser_close',
    serviceName,
    agentName,
    taskName,
    params: { browserId },
    jobTimeoutMs: 30000,
  });
  assert(closeResponse.success === true, `service_browser_close failed: ${JSON.stringify(closeResponse)}`);
  assert(closeResponse.data?.closed === true, `service_browser_close did not report closed: ${JSON.stringify(closeResponse)}`);
  assert(closeResponse.data?.browserId === browserId, `service_browser_close browserId mismatch: ${JSON.stringify(closeResponse)}`);
  assert(closeResponse.data?.requestedBrowserId === browserId, `service_browser_close requestedBrowserId mismatch: ${JSON.stringify(closeResponse)}`);
  assert(closeResponse.data?.serviceOwned === true, `service_browser_close missing serviceOwned=true: ${JSON.stringify(closeResponse)}`);

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status after row close');
  assert(status.success === true, `Service status failed: ${statusResult.stdout}${statusResult.stderr}`);

  const browser = status.data?.service_state?.browsers?.[browserId];
  assert(browser, `Service state missing browser ${browserId}: ${JSON.stringify(status.data)}`);
  assert(
    browser.health === 'not_started',
    `Polite service_browser_close should leave browser not_started, got ${browser.health}: ${JSON.stringify(browser)}`,
  );
  assert(
    browser.lastError === null || browser.lastError === undefined,
    `Successful service_browser_close should not retain lastError: ${JSON.stringify(browser)}`,
  );

  const sessionRecord = status.data?.service_state?.sessions?.[session];
  assert(sessionRecord, `Service state missing session ${session}: ${JSON.stringify(status.data)}`);
  assert(sessionRecord.lease === 'released', `Closed browser did not release lease: ${JSON.stringify(sessionRecord)}`);
  assert(
    Array.isArray(sessionRecord.profileLeaseConflictSessionIds) &&
      sessionRecord.profileLeaseConflictSessionIds.length === 0,
    `Closed browser retained profile lease conflicts: ${JSON.stringify(sessionRecord)}`,
  );

  const events = status.data?.service_state?.events ?? [];
  const closeEvent = events.find(
    (event) =>
      event.kind === 'browser_health_changed' &&
      event.browserId === browserId &&
      event.currentHealth === 'not_started',
  );
  assert(closeEvent, `Service events missing successful close health transition: ${JSON.stringify(events)}`);
  assert(
    closeEvent.details?.shutdownReasonKind === 'operator_requested_close' &&
      closeEvent.details?.processExitCause === 'operator_requested_close' &&
      closeEvent.details?.shutdownRequested === true,
    `Successful close event missing operator-requested metadata: ${JSON.stringify(closeEvent)}`,
  );
  assert(
    closeEvent.details?.politeCloseAttempted === true &&
      closeEvent.details?.politeCloseFailed !== true &&
      closeEvent.details?.forceKillFailed !== true,
    `Successful close event recorded failed shutdown metadata: ${JSON.stringify(closeEvent)}`,
  );

  await cleanup();
  console.log('Service browser row close live smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
