#!/usr/bin/env node

import {
  assertServiceRemediesTextOutput,
  assertServiceStatusDidNotLaunch,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
  seedIncidentSummarySmokeState,
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

try {
  await seedIncidentSummarySmokeState(context, { serviceName, agentName, taskName });
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

  assertServiceRemediesTextOutput(remedies.stdout, 'service remedies text output');

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status after remedies');
  assertServiceStatusDidNotLaunch(status, 'service remedies');

  await cleanup();
  console.log('Service remedies CLI no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
