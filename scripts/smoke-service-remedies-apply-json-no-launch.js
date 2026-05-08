#!/usr/bin/env node

import {
  assertBrowserDegradedRemediesApplyJsonResponse,
  assertServiceStatusDidNotLaunch,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
  seedDegradedRemedySmokeBrowser,
  seedIncidentSummarySmokeState,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-remedies-apply-json-no-launch-',
  sessionPrefix: 'remedies-apply-json-no-launch',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'RemediesApplyJsonSmoke';
const agentName = 'smoke-agent';
const taskName = 'applyBrowserDegradedRemedy';

async function cleanup() {
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

try {
  await seedIncidentSummarySmokeState(context, { serviceName, agentName, taskName });
  seedDegradedRemedySmokeBrowser(context);

  const result = await runCli(context, [
    '--json',
    '--session',
    session,
    'service',
    'remedies',
    'apply',
    '--escalation',
    'browser_degraded',
    '--by',
    'operator',
    '--note',
    'reviewed',
  ]);
  const apply = parseJsonOutput(result.stdout, 'service remedies apply');

  if (apply.success !== true) {
    throw new Error(`service remedies apply JSON failed: ${result.stdout}${result.stderr}`);
  }
  assertBrowserDegradedRemediesApplyJsonResponse(apply.data, 'service remedies apply JSON output');

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status after remedies apply');
  assertServiceStatusDidNotLaunch(status, 'service remedies apply JSON');

  const browser = status.data?.service_state?.browsers?.['browser-summary-degraded'];
  assertBrowserDegradedRemediesApplyJsonResponse(
    {
      applied: true,
      escalation: 'browser_degraded',
      count: 1,
      monitorIds: [],
      monitorResults: [],
      browserIds: ['browser-summary-degraded'],
      browserResults: [
        {
          id: 'browser-summary-degraded',
          retryEnabled: true,
          browser,
          incident: apply.data.browserResults[0].incident,
        },
      ],
    },
    'persisted service remedies apply browser state',
  );

  await cleanup();
  console.log('Service remedies apply JSON no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
