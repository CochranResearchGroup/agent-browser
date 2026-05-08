#!/usr/bin/env node

import {
  assertServiceRemediesJsonResponse,
  assertServiceStatusDidNotLaunch,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
  seedIncidentSummarySmokeState,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-remedies-json-no-launch-',
  sessionPrefix: 'remedies-json-no-launch',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'RemediesJsonSmoke';
const agentName = 'smoke-agent';
const taskName = 'renderJsonRemedyLadder';

async function cleanup() {
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

try {
  await seedIncidentSummarySmokeState(context, { serviceName, agentName, taskName });
  const result = await runCli(context, [
    '--json',
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
  const remedies = parseJsonOutput(result.stdout, 'service remedies');

  if (remedies.success !== true) {
    throw new Error(`service remedies JSON failed: ${result.stdout}${result.stderr}`);
  }
  assertServiceRemediesJsonResponse(remedies.data, 'service remedies JSON output');

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status after JSON remedies');
  assertServiceStatusDidNotLaunch(status, 'service remedies JSON');

  await cleanup();
  console.log('Service remedies JSON no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
