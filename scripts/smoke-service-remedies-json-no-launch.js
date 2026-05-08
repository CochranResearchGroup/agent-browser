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

async function seedIncidentEvents() {
  const result = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(result.stdout, 'service status');
  assert(status.success === true, `service status failed before seed: ${result.stdout}${result.stderr}`);
  assert(
    status.data?.control_plane?.browser_health === 'NotStarted',
    `service status launched browser before JSON remedies smoke: ${JSON.stringify(status.data?.control_plane)}`,
  );
  seedIncidentSummarySmokeEvents(context, { serviceName, agentName, taskName });
}

function findGroup(groups, escalation) {
  return groups.find((group) => group.escalation === escalation);
}

function assertIncludesAll(values, expected, label) {
  for (const value of expected) {
    assert(values.includes(value), `${label} missing ${value}: ${JSON.stringify(values)}`);
  }
}

try {
  await seedIncidentEvents();
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

  assert(remedies.success === true, `service remedies JSON failed: ${result.stdout}${result.stderr}`);
  assert(remedies.data?.count === 3, `service remedies JSON count mismatch: ${result.stdout}`);
  assert(remedies.data?.matched === 3, `service remedies JSON matched mismatch: ${result.stdout}`);
  assert(remedies.data?.filters?.remediesOnly === true, `service remedies JSON missing remediesOnly filter: ${result.stdout}`);
  assert(remedies.data?.filters?.state === 'active', `service remedies JSON missing active state filter: ${result.stdout}`);
  assert(remedies.data?.summary?.groupCount === 2, `service remedies JSON group count mismatch: ${result.stdout}`);

  const groups = remedies.data.summary.groups;
  assert(Array.isArray(groups), `service remedies JSON missing groups: ${result.stdout}`);

  const degraded = findGroup(groups, 'browser_degraded');
  assert(degraded, `service remedies JSON missing browser_degraded group: ${result.stdout}`);
  assert(degraded.severity === 'warning', `browser_degraded severity mismatch: ${JSON.stringify(degraded)}`);
  assert(degraded.state === 'active', `browser_degraded state mismatch: ${JSON.stringify(degraded)}`);
  assert(degraded.count === 1, `browser_degraded count mismatch: ${JSON.stringify(degraded)}`);
  assertIncludesAll(degraded.browserIds, ['browser-summary-degraded'], 'browser_degraded browserIds');
  assertIncludesAll(degraded.incidentIds, ['browser-summary-degraded'], 'browser_degraded incidentIds');
  assert(
    degraded.remedyApplyCommand === 'agent-browser service remedies apply --escalation browser_degraded',
    `browser_degraded apply command mismatch: ${JSON.stringify(degraded)}`,
  );

  const osDegraded = findGroup(groups, 'os_degraded_possible');
  assert(osDegraded, `service remedies JSON missing os_degraded_possible group: ${result.stdout}`);
  assert(osDegraded.severity === 'critical', `os_degraded_possible severity mismatch: ${JSON.stringify(osDegraded)}`);
  assert(osDegraded.state === 'active', `os_degraded_possible state mismatch: ${JSON.stringify(osDegraded)}`);
  assert(osDegraded.count === 2, `os_degraded_possible count mismatch: ${JSON.stringify(osDegraded)}`);
  assertIncludesAll(
    osDegraded.browserIds,
    ['browser-summary-faulted-1', 'browser-summary-faulted-2'],
    'os_degraded_possible browserIds',
  );
  assertIncludesAll(
    osDegraded.incidentIds,
    ['browser-summary-faulted-1', 'browser-summary-faulted-2'],
    'os_degraded_possible incidentIds',
  );
  assert(
    osDegraded.remedyApplyCommand === 'agent-browser service remedies apply --escalation os_degraded_possible',
    `os_degraded_possible apply command mismatch: ${JSON.stringify(osDegraded)}`,
  );

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status after JSON remedies');
  assert(
    status.data?.control_plane?.browser_health === 'NotStarted',
    `service remedies JSON launched browser: ${JSON.stringify(status.data?.control_plane)}`,
  );

  await cleanup();
  console.log('Service remedies JSON no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
