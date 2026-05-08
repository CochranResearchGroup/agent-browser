#!/usr/bin/env node

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
  seedIncidentSummarySmokeEvents,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-remedies-cli-no-launch-',
  sessionPrefix: 'remedies-cli-no-launch',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'RemediesCliSmoke';
const agentName = 'smoke-agent';
const taskName = 'renderCliRemedyLadder';

async function cleanup() {
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

async function seedIncidentEvents() {
  const result = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(result.stdout, 'service status');
  assert(status.success === true, `service status failed before seed: ${result.stdout}${result.stderr}`);
  assert(
    status.data?.control_plane?.browser_health === 'NotStarted',
    `service status launched browser before remedies smoke: ${JSON.stringify(status.data?.control_plane)}`,
  );
  seedIncidentSummarySmokeEvents(context, { serviceName, agentName, taskName });
}

try {
  await seedIncidentEvents();
  const remedies = await runCli(context, [
    '--session',
    session,
    'service',
    'remedies',
    '--service-name',
    serviceName,
    '--agent-name',
    agentName,
    '--task-name',
    taskName,
  ]);

  const output = remedies.stdout;
  assert(output.includes('Incident groups: 2'), `remedies output missing group count:\n${output}`);
  assert(
    output.includes('critical escalation=os_degraded_possible state=active count=2'),
    `remedies output missing OS-degraded group:\n${output}`,
  );
  assert(
    output.includes('warning escalation=browser_degraded state=active count=1'),
    `remedies output missing degraded-browser group:\n${output}`,
  );
  assert(
    output.includes('browsers=browser-summary-faulted-1,browser-summary-faulted-2'),
    `remedies output missing affected faulted browsers:\n${output}`,
  );
  assert(
    output.includes('browsers=browser-summary-degraded'),
    `remedies output missing affected degraded browser:\n${output}`,
  );
  assert(
    output.includes('apply=agent-browser service remedies apply --escalation os_degraded_possible'),
    `remedies output missing OS-degraded apply command:\n${output}`,
  );
  assert(
    output.includes('apply=agent-browser service remedies apply --escalation browser_degraded'),
    `remedies output missing degraded-browser apply command:\n${output}`,
  );

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status after remedies');
  assert(
    status.data?.control_plane?.browser_health === 'NotStarted',
    `service remedies launched browser: ${JSON.stringify(status.data?.control_plane)}`,
  );

  await cleanup();
  console.log('Service remedies CLI no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
