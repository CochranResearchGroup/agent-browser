#!/usr/bin/env node

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-ssh-', sessionPrefix: 'ssh' });
const { session } = context;

context.env.AGENT_BROWSER_TEST_FORCE_POLITE_CLOSE_FAILURE = '1';

const timeout = setTimeout(() => {
  fail('Timed out waiting for service shutdown health smoke to complete');
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
      dataUrl('Shutdown Health Smoke', 'Shutdown Health Smoke'),
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
    browser.health === 'degraded',
    `Polite close failure should leave browser degraded, got ${browser.health}: ${JSON.stringify(browser)}`,
  );
  assert(
    typeof browser.lastError === 'string' &&
      browser.lastError.includes('Polite browser close failed') &&
      browser.lastError.includes('force kill was required'),
    `Degraded browser did not record shutdown remedy details: ${JSON.stringify(browser)}`,
  );

  const events = status.data?.service_state?.events ?? [];
  assert(Array.isArray(events), 'Service state missing events array');
  const degradedEvent = events.find(
    (event) =>
      event.kind === 'browser_health_changed' &&
      event.browserId === browserId &&
      event.currentHealth === 'degraded',
  );
  assert(
    degradedEvent,
    `Service events missing degraded shutdown health transition: ${JSON.stringify(events)}`,
  );
  assert(
    degradedEvent.details?.shutdownReasonKind === 'operator_requested_close' &&
      degradedEvent.details?.shutdownRequested === true,
    `Degraded shutdown event missing operator-requested close metadata: ${JSON.stringify(degradedEvent)}`,
  );
  assert(
    degradedEvent.details?.politeCloseFailed === true &&
      degradedEvent.details?.forceKillAttempted === true &&
      degradedEvent.details?.forceKillSucceeded === true,
    `Degraded shutdown event missing shutdown outcome metadata: ${JSON.stringify(degradedEvent)}`,
  );

  const incidents = status.data?.service_state?.incidents ?? [];
  assert(Array.isArray(incidents), 'Service state missing incidents array');
  const incident = incidents.find((item) => item.id === browserId);
  assert(incident, `Service incidents missing ${browserId}: ${JSON.stringify(incidents)}`);
  assert(
    incident.severity === 'warning',
    `Degraded browser incident should be warning severity: ${JSON.stringify(incident)}`,
  );
  assert(
    incident.escalation === 'browser_degraded',
    `Degraded browser incident should use browser_degraded escalation: ${JSON.stringify(incident)}`,
  );
  assert(
    typeof incident.recommendedAction === 'string' && incident.recommendedAction.includes('browser health'),
    `Degraded browser incident missing recommendedAction: ${JSON.stringify(incident)}`,
  );

  await cleanup();
  console.log('Service shutdown health smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
