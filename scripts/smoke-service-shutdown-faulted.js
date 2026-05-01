#!/usr/bin/env node

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-ssf-', sessionPrefix: 'ssf' });
const { session } = context;

context.env.AGENT_BROWSER_TEST_FORCE_POLITE_CLOSE_FAILURE = '1';
context.env.AGENT_BROWSER_TEST_FORCE_KILL_FAILURE = '1';

const timeout = setTimeout(() => {
  fail('Timed out waiting for service shutdown faulted smoke to complete');
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
      dataUrl('Shutdown Faulted Smoke', 'Shutdown Faulted Smoke'),
    ],
    180000,
  );
  const opened = parseJsonOutput(openResult.stdout, 'open');
  assert(opened.success === true, `Open command failed: ${openResult.stdout}${openResult.stderr}`);

  const closeResult = await runCli(context, ['--json', '--session', session, 'close']);
  const closed = parseJsonOutput(closeResult.stdout, 'close');
  assert(closed.success === true, `Close command failed: ${closeResult.stdout}${closeResult.stderr}`);

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status');
  assert(status.success === true, `Service status failed: ${statusResult.stdout}${statusResult.stderr}`);

  const browserId = `session:${session}`;
  const browser = status.data?.service_state?.browsers?.[browserId];
  assert(browser, `Service state missing browser ${browserId}: ${JSON.stringify(status.data)}`);
  assert(
    browser.health === 'faulted',
    `Force kill failure should leave browser faulted, got ${browser.health}: ${JSON.stringify(browser)}`,
  );
  assert(
    typeof browser.lastError === 'string' &&
      browser.lastError.includes('Force kill failed') &&
      browser.lastError.includes('OS may be degraded'),
    `Faulted browser did not record OS degradation details: ${JSON.stringify(browser)}`,
  );
  assert(
    browser.lastHealthObservation?.failureClass === 'browser_shutdown_force_kill_failed' &&
      browser.lastHealthObservation?.processExitCause === 'operator_requested_close',
    `Faulted browser did not retain last health observation: ${JSON.stringify(browser)}`,
  );

  const browsersResult = await runCli(context, ['--json', '--session', session, 'service', 'browsers']);
  const browsersResponse = parseJsonOutput(browsersResult.stdout, 'service browsers');
  assert(
    browsersResponse.success === true,
    `Service browsers failed: ${browsersResult.stdout}${browsersResult.stderr}`,
  );
  const collectionBrowser = browsersResponse.data?.browsers?.find((item) => item.id === browserId);
  assert(
    collectionBrowser?.lastHealthObservation?.failureClass === 'browser_shutdown_force_kill_failed' &&
      collectionBrowser?.lastHealthObservation?.processExitCause === 'operator_requested_close',
    `Service browsers did not expose retained fault evidence: ${JSON.stringify(browsersResponse.data)}`,
  );

  const events = status.data?.service_state?.events ?? [];
  assert(Array.isArray(events), 'Service state missing events array');
  const faultedEvent = events.find(
    (event) =>
      event.kind === 'browser_health_changed' &&
      event.browserId === browserId &&
      event.currentHealth === 'faulted',
  );
  assert(faultedEvent, `Service events missing faulted shutdown health transition: ${JSON.stringify(events)}`);
  assert(
    faultedEvent.details?.shutdownReasonKind === 'operator_requested_close' &&
      faultedEvent.details?.processExitCause === 'operator_requested_close' &&
      faultedEvent.details?.shutdownRequested === true,
    `Faulted shutdown event missing operator-requested close metadata: ${JSON.stringify(faultedEvent)}`,
  );
  assert(
    faultedEvent.details?.politeCloseFailed === true &&
      faultedEvent.details?.forceKillAttempted === true &&
      faultedEvent.details?.forceKillSucceeded === false &&
      faultedEvent.details?.forceKillFailed === true,
    `Faulted shutdown event missing shutdown outcome metadata: ${JSON.stringify(faultedEvent)}`,
  );

  const incidents = status.data?.service_state?.incidents ?? [];
  assert(Array.isArray(incidents), 'Service state missing incidents array');
  const incident = incidents.find((item) => item.id === browserId);
  assert(incident, `Service incidents missing ${browserId}: ${JSON.stringify(incidents)}`);
  assert(
    incident.severity === 'critical',
    `Faulted browser incident should be critical severity: ${JSON.stringify(incident)}`,
  );
  assert(
    incident.escalation === 'os_degraded_possible',
    `Faulted browser incident should use os_degraded_possible escalation: ${JSON.stringify(incident)}`,
  );
  assert(
    typeof incident.recommendedAction === 'string' &&
      incident.recommendedAction.includes('Inspect the host OS'),
    `Faulted browser incident missing OS recommendedAction: ${JSON.stringify(incident)}`,
  );

  await cleanup();
  console.log('Service shutdown faulted smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
