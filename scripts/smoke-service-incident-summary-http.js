#!/usr/bin/env node

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import {
  assertServiceIncidentsResponseSchemaRecord,
  loadServiceRecordSchema,
} from './smoke-schema-utils.js';

const context = createSmokeContext({
  prefix: 'ab-incident-summary-http-',
  sessionPrefix: 'incident-summary-http',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { agentHome, session } = context;
const serviceName = 'IncidentSummaryHttpSmoke';
const agentName = 'smoke-agent';
const taskName = 'groupHttpIncidentRemedies';
const incidentsResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-incidents-response.v1.schema.json',
);

async function cleanup() {
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

async function enableStream() {
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
  return port;
}

function seedIncidentEvents() {
  const statusResult = runCli(context, ['--json', '--session', session, 'service', 'status']);
  return statusResult.then((result) => {
    const status = parseJsonOutput(result.stdout, 'service status');
    assert(status.success === true, `service status failed before seed: ${result.stdout}${result.stderr}`);

    const serviceDir = join(agentHome, 'service');
    const statePath = join(serviceDir, 'state.json');
    mkdirSync(serviceDir, { recursive: true });
    const state = JSON.parse(readFileSync(statePath, 'utf8'));
    state.events = [
      {
        id: 'event-summary-critical-1',
        timestamp: '2026-05-01T10:00:00Z',
        kind: 'browser_health_changed',
        message: 'Browser browser-summary-faulted-1 faulted',
        browserId: 'browser-summary-faulted-1',
        profileId: 'summary-profile',
        sessionId: session,
        serviceName,
        agentName,
        taskName,
        previousHealth: 'ready',
        currentHealth: 'faulted',
        details: { failureClass: 'force_kill_failed' },
      },
      {
        id: 'event-summary-critical-2',
        timestamp: '2026-05-01T10:01:00Z',
        kind: 'browser_health_changed',
        message: 'Browser browser-summary-faulted-2 faulted',
        browserId: 'browser-summary-faulted-2',
        profileId: 'summary-profile',
        sessionId: session,
        serviceName,
        agentName,
        taskName,
        previousHealth: 'ready',
        currentHealth: 'faulted',
        details: { failureClass: 'force_kill_failed' },
      },
      {
        id: 'event-summary-warning',
        timestamp: '2026-05-01T10:02:00Z',
        kind: 'browser_health_changed',
        message: 'Browser browser-summary-degraded degraded',
        browserId: 'browser-summary-degraded',
        profileId: 'summary-profile',
        sessionId: session,
        serviceName,
        agentName,
        taskName,
        previousHealth: 'ready',
        currentHealth: 'degraded',
        details: { failureClass: 'polite_close_failed' },
      },
    ];
    writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`);
  });
}

function findSummaryGroup(summary, escalation, severity, state) {
  return summary.groups.find(
    (group) =>
      group.escalation === escalation &&
      group.severity === severity &&
      group.state === state,
  );
}

function assertSummaryShape(summary) {
  assert(summary && typeof summary === 'object', `HTTP incidents missing summary object: ${JSON.stringify(summary)}`);
  assert(Number.isInteger(summary.groupCount), `HTTP incidents summary missing groupCount: ${JSON.stringify(summary)}`);
  assert(Array.isArray(summary.groups), `HTTP incidents summary missing groups: ${JSON.stringify(summary)}`);
  assert(summary.groupCount === summary.groups.length, `HTTP incidents summary count mismatch: ${JSON.stringify(summary)}`);

  const critical = findSummaryGroup(summary, 'os_degraded_possible', 'critical', 'active');
  assert(critical, `HTTP incidents summary missing critical OS group: ${JSON.stringify(summary)}`);
  assert(critical.count === 2, `HTTP incidents summary critical count mismatch: ${JSON.stringify(critical)}`);
  assert(
    critical.incidentIds.includes('browser-summary-faulted-1') &&
      critical.incidentIds.includes('browser-summary-faulted-2'),
    `HTTP incidents summary critical IDs mismatch: ${JSON.stringify(critical)}`,
  );
  assert(
    critical.recommendedAction.includes('host OS'),
    `HTTP incidents summary critical remedy mismatch: ${JSON.stringify(critical)}`,
  );

  const warning = findSummaryGroup(summary, 'browser_degraded', 'warning', 'active');
  assert(warning, `HTTP incidents summary missing degraded-browser group: ${JSON.stringify(summary)}`);
  assert(warning.count === 1, `HTTP incidents summary warning count mismatch: ${JSON.stringify(warning)}`);
  assert(
    warning.incidentIds.includes('browser-summary-degraded'),
    `HTTP incidents summary warning IDs mismatch: ${JSON.stringify(warning)}`,
  );
}

try {
  await seedIncidentEvents();
  const port = await enableStream();
  const incidents = await httpJson(
    port,
    'GET',
    `/api/service/incidents?summary=true&limit=20&service-name=${encodeURIComponent(
      serviceName,
    )}&agent-name=${encodeURIComponent(agentName)}&task-name=${encodeURIComponent(taskName)}`,
  );

  assert(incidents.success === true, `HTTP service incidents failed: ${JSON.stringify(incidents)}`);
  assertServiceIncidentsResponseSchemaRecord(
    incidents.data,
    incidentsResponseSchema,
    'HTTP incidents summary response',
  );
  assert(incidents.data.count === 3, `HTTP incidents summary response count mismatch: ${JSON.stringify(incidents)}`);
  assertSummaryShape(incidents.data.summary);

  await cleanup();
  console.log('Service incident HTTP summary smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
