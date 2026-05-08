#!/usr/bin/env node

import {
  assertBrowserDegradedRemediesApplyJsonResponse,
  assertOsDegradedRemediesApplyJsonResponse,
  assertServiceStatusDidNotLaunch,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
  seedDegradedRemedySmokeBrowser,
  seedIncidentSummarySmokeState,
  seedOsDegradedRemedySmokeBrowsers,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-remedies-apply-json-no-launch-',
  sessionPrefix: 'remedies-apply-json-no-launch',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'RemediesApplyJsonSmoke';
const agentName = 'smoke-agent';
const taskName = 'applyBrowserRemedies';

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
  seedOsDegradedRemedySmokeBrowsers(context);

  const degradedResult = await runCli(context, [
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
  const degradedApply = parseJsonOutput(degradedResult.stdout, 'service remedies apply browser_degraded');

  if (degradedApply.success !== true) {
    throw new Error(`service remedies apply browser_degraded JSON failed: ${degradedResult.stdout}${degradedResult.stderr}`);
  }
  assertBrowserDegradedRemediesApplyJsonResponse(
    degradedApply.data,
    'service remedies apply browser_degraded JSON output',
  );

  const osResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'service',
    'remedies',
    'apply',
    '--escalation',
    'os_degraded_possible',
    '--by',
    'operator',
    '--note',
    'host inspected',
  ]);
  const osApply = parseJsonOutput(osResult.stdout, 'service remedies apply os_degraded_possible');

  if (osApply.success !== true) {
    throw new Error(`service remedies apply os_degraded_possible JSON failed: ${osResult.stdout}${osResult.stderr}`);
  }
  assertOsDegradedRemediesApplyJsonResponse(
    osApply.data,
    'service remedies apply os_degraded_possible JSON output',
  );

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
          incident: degradedApply.data.browserResults[0].incident,
        },
      ],
    },
    'persisted service remedies apply browser state',
  );

  const browsers = status.data?.service_state?.browsers ?? {};
  assertOsDegradedRemediesApplyJsonResponse(
    {
      applied: true,
      escalation: 'os_degraded_possible',
      count: 2,
      monitorIds: [],
      monitorResults: [],
      browserIds: ['browser-summary-faulted-1', 'browser-summary-faulted-2'],
      browserResults: osApply.data.browserResults.map((result) => ({
        id: result.id,
        retryEnabled: true,
        browser: browsers[result.id],
        incident: result.incident,
      })),
    },
    'persisted service remedies apply OS-degraded browser state',
  );

  await cleanup();
  console.log('Service remedies apply JSON no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
