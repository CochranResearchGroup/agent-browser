#!/usr/bin/env node

import {
  appendPriorRecoveryAttempt,
  assert,
  assertHttpMcpServiceTraceEventParity,
  assertRecoveryBudgetBlockedEvents,
  closeSession,
  configureRecoveryOverrideSmokeContext,
  createMcpStdioClient,
  createSmokeContext,
  httpJson,
  httpJsonResult,
  parseJsonOutput,
  parseMcpToolPayload,
  recoveryOverrideSmokeUrls,
  runCli,
} from './smoke-utils.js';
import {
  assertServiceEventSchemaRecord,
  assertServiceIncidentSchemaRecord,
  assertServiceTraceResponseSchemaRecord,
  assertServiceTraceActivitySchemaRecord,
  assertServiceTraceSummarySchemaRecord,
  loadServiceRecordSchema,
  parseMcpJsonResource,
} from './smoke-schema-utils.js';

const context = configureRecoveryOverrideSmokeContext(
  createSmokeContext({ prefix: 'ab-sip-', sessionPrefix: 'sip' }),
);

const { session } = context;
const serviceName = 'IncidentParitySmoke';
const agentName = 'smoke-agent';
const taskName = 'compareHttpMcpIncidents';
const traceFields = { serviceName, agentName, taskName };
const incidentRecordSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-incident-record.v1.schema.json',
);
const eventRecordSchema = loadServiceRecordSchema('../docs/dev/contracts/service-event-record.v1.schema.json');
const traceResponseRecordSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-trace-response.v1.schema.json',
);
const traceSummaryRecordSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-trace-summary-record.v1.schema.json',
);
const traceActivityRecordSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-trace-activity-record.v1.schema.json',
);

let mcp;
const timeout = setTimeout(() => {
  fail('Timed out waiting for service incident parity smoke to complete');
}, 120000);

function send(method, params) {
  return mcp.send(method, params);
}

function notify(method, params) {
  mcp.notify(method, params);
}

async function cleanup() {
  clearTimeout(timeout);
  if (mcp) mcp.close();
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

async function fail(message) {
  if (mcp) mcp.rejectPending(message);
  console.error(message);
  const stderr = mcp?.stderr() ?? '';
  if (stderr.trim()) console.error(stderr.trim());
  await cleanup();
  process.exit(1);
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

async function command(port, body) {
  const response = await httpJson(port, 'POST', '/api/command', { ...body, ...traceFields });
  assert(response.success === true, `HTTP command ${body.action} failed: ${JSON.stringify(response)}`);
  return response;
}

function assertCriticalIncidentPayload(payload, label, browserId) {
  assert(payload.success === true, `${label} incidents query failed: ${JSON.stringify(payload)}`);
  assert(Array.isArray(payload.data?.incidents), `${label} missing incidents array`);
  assert(payload.data.filters?.severity === 'critical', `${label} did not retain severity filter`);
  assert(
    payload.data.filters?.escalation === 'os_degraded_possible',
    `${label} did not retain escalation filter`,
  );
  const incident = payload.data.incidents.find((item) => item.id === browserId);
  assert(incident, `${label} missing ${browserId}: ${JSON.stringify(payload.data.incidents)}`);
  assertServiceIncidentSchemaRecord(incident, incidentRecordSchema, label);
  assert(incident.severity === 'critical', `${label} incident severity mismatch: ${JSON.stringify(incident)}`);
  assert(
    incident.escalation === 'os_degraded_possible',
    `${label} incident escalation mismatch: ${JSON.stringify(incident)}`,
  );
  assert(incident.currentHealth === 'faulted', `${label} incident health mismatch: ${JSON.stringify(incident)}`);
  assert(
    typeof incident.recommendedAction === 'string' &&
      incident.recommendedAction.includes('host OS'),
    `${label} incident recommendedAction mismatch: ${JSON.stringify(incident)}`,
  );
  return incident;
}

try {
  const { blockedUrl, initialUrl } = recoveryOverrideSmokeUrls('Incident Parity Smoke');

  const openResult = await runCli(context, [
    '--json',
    '--session',
    session,
    '--args',
    '--no-sandbox',
    'open',
    initialUrl,
  ]);
  const opened = parseJsonOutput(openResult.stdout, 'open');
  assert(opened.success === true, `Open command failed: ${openResult.stdout}${openResult.stderr}`);

  const port = await enableStream();

  await command(port, {
    id: 'service-incident-parity-smoke-launch',
    action: 'launch',
    headless: true,
    args: ['--no-sandbox'],
  });

  mcp = createMcpStdioClient({
    context,
    args: ['--session', session, 'mcp', 'serve'],
    onFatal: fail,
  });
  const initialize = await send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'agent-browser-service-incident-parity-smoke', version: '0' },
  });
  assert(initialize.capabilities?.tools, 'MCP tools capability missing');
  notify('notifications/initialized');

  const pidResponse = await command(port, {
    id: 'service-incident-parity-smoke-browser-pid',
    action: 'browser_pid',
  });
  const pid = pidResponse.data?.pid;
  assert(Number.isInteger(pid) && pid > 0, `browser_pid did not return a pid: ${JSON.stringify(pidResponse)}`);

  const browserId = `session:${session}`;
  appendPriorRecoveryAttempt(context, {
    agentName,
    browserId,
    serviceName,
    taskName,
  });
  process.kill(pid, 'SIGKILL');

  const blocked = await httpJsonResult(port, 'POST', '/api/browser/navigate', {
    url: blockedUrl,
    ...traceFields,
  });
  assert(blocked.statusCode === 200, `HTTP browser navigate returned ${blocked.statusCode}`);
  assert(
    blocked.body?.success === false &&
      typeof blocked.body?.error === 'string' &&
      blocked.body.error.includes('retry budget exceeded'),
    `HTTP browser navigate did not report retry budget exhaustion: ${JSON.stringify(blocked)}`,
  );

  const trace = await httpJson(
    port,
    'GET',
    `/api/service/trace?service-name=${encodeURIComponent(serviceName)}&agent-name=${encodeURIComponent(
      agentName,
    )}&task-name=${encodeURIComponent(taskName)}&limit=80`,
  );
  assert(trace.success === true, `HTTP service trace failed: ${JSON.stringify(trace)}`);

  const mcpTraceResult = await send('tools/call', {
    name: 'service_trace',
    arguments: {
      serviceName,
      agentName,
      taskName,
      limit: 80,
    },
  });
  const mcpTrace = parseMcpToolPayload(mcpTraceResult, 'MCP service_trace');
  const { httpEvent, mcpEvent } = assertHttpMcpServiceTraceEventParity({
    httpTrace: trace,
    mcpTrace,
    label: 'incident parity trace',
    assertEvent: (events, label) =>
      assertRecoveryBudgetBlockedEvents(events, { browserId, label }).faultedEvent,
  });
  assertServiceTraceResponseSchemaRecord(trace.data, traceResponseRecordSchema, 'HTTP trace response');
  assertServiceTraceResponseSchemaRecord(mcpTrace.data, traceResponseRecordSchema, 'MCP trace response');
  assertServiceEventSchemaRecord(httpEvent, eventRecordSchema, 'HTTP trace event');
  assertServiceEventSchemaRecord(mcpEvent, eventRecordSchema, 'MCP trace event');
  assertServiceTraceSummarySchemaRecord(trace.data.summary, traceSummaryRecordSchema, 'HTTP trace summary');
  assertServiceTraceSummarySchemaRecord(mcpTrace.data.summary, traceSummaryRecordSchema, 'MCP trace summary');
  for (const [index, item] of trace.data.activity.entries()) {
    assertServiceTraceActivitySchemaRecord(item, traceActivityRecordSchema, `HTTP trace activity[${index}]`);
  }
  for (const [index, item] of mcpTrace.data.activity.entries()) {
    assertServiceTraceActivitySchemaRecord(item, traceActivityRecordSchema, `MCP trace activity[${index}]`);
  }

  const httpEvents = await httpJson(
    port,
    'GET',
    `/api/service/events?kind=browser_health_changed&browser-id=${encodeURIComponent(browserId)}&limit=20`,
  );
  assert(httpEvents.success === true, `HTTP service events failed: ${JSON.stringify(httpEvents)}`);
  const httpEventsRecord = httpEvents.data?.events?.find((event) => event.id === httpEvent.id);
  assert(httpEventsRecord, `HTTP events missing schema event ${httpEvent.id}: ${JSON.stringify(httpEvents)}`);
  assertServiceEventSchemaRecord(httpEventsRecord, eventRecordSchema, 'HTTP events record');

  const httpIncidents = await httpJson(
    port,
    'GET',
    `/api/service/incidents?severity=critical&escalation=os_degraded_possible&service-name=${encodeURIComponent(
      serviceName,
    )}&agent-name=${encodeURIComponent(agentName)}&task-name=${encodeURIComponent(taskName)}&limit=20`,
  );
  const httpIncident = assertCriticalIncidentPayload(httpIncidents, 'HTTP', browserId);

  const httpIncidentDetail = await httpJson(
    port,
    'GET',
    `/api/service/incidents/${encodeURIComponent(browserId)}?limit=20`,
  );
  assert(httpIncidentDetail.success === true, `HTTP incident detail failed: ${JSON.stringify(httpIncidentDetail)}`);
  assert(httpIncidentDetail.data?.incident?.id === browserId, 'HTTP incident detail returned the wrong incident');
  assertServiceIncidentSchemaRecord(httpIncidentDetail.data.incident, incidentRecordSchema, 'HTTP detail');

  const mcpIncidentsResult = await send('tools/call', {
    name: 'service_incidents',
    arguments: {
      severity: 'critical',
      escalation: 'os_degraded_possible',
      serviceName,
      agentName,
      taskName,
      limit: 20,
    },
  });
  const mcpIncidents = parseMcpToolPayload(mcpIncidentsResult, 'MCP service_incidents');
  const mcpIncident = assertCriticalIncidentPayload(mcpIncidents, 'MCP', browserId);

  const mcpResourceResult = await send('resources/read', { uri: 'agent-browser://incidents' });
  const mcpResourcePayload = parseMcpJsonResource(
    mcpResourceResult,
    'agent-browser://incidents',
    'MCP incidents resource',
  );
  assert(Array.isArray(mcpResourcePayload.incidents), 'MCP incidents resource missing incidents array');
  const mcpResourceIncident = mcpResourcePayload.incidents.find((item) => item.id === browserId);
  assert(
    mcpResourceIncident,
    `MCP incidents resource missing ${browserId}: ${JSON.stringify(mcpResourcePayload.incidents)}`,
  );
  assertServiceIncidentSchemaRecord(mcpResourceIncident, incidentRecordSchema, 'MCP resource');

  const mcpEventsResource = await send('resources/read', { uri: 'agent-browser://events' });
  const mcpEventsPayload = parseMcpJsonResource(mcpEventsResource, 'agent-browser://events', 'MCP events resource');
  assert(Array.isArray(mcpEventsPayload.events), 'MCP events resource missing events array');
  const mcpResourceEvent = mcpEventsPayload.events.find((event) => event.id === httpEvent.id);
  assert(
    mcpResourceEvent,
    `MCP events resource missing ${httpEvent.id}: ${JSON.stringify(mcpEventsPayload.events)}`,
  );
  assertServiceEventSchemaRecord(mcpResourceEvent, eventRecordSchema, 'MCP events resource');

  const traceIncident = trace.data.incidents.find((item) => item.id === browserId);
  assert(
    traceIncident,
    `HTTP trace missing schema incident ${browserId}: ${JSON.stringify(trace.data.incidents)}`,
  );
  assertServiceIncidentSchemaRecord(traceIncident, incidentRecordSchema, 'HTTP trace');
  const mcpTraceIncident = mcpTrace.data.incidents.find((item) => item.id === browserId);
  assert(
    mcpTraceIncident,
    `MCP trace missing schema incident ${browserId}: ${JSON.stringify(mcpTrace.data.incidents)}`,
  );
  assertServiceIncidentSchemaRecord(mcpTraceIncident, incidentRecordSchema, 'MCP trace');

  for (const field of ['id', 'severity', 'escalation', 'currentHealth', 'latestKind', 'recommendedAction']) {
    assert(
      mcpIncident[field] === httpIncident[field],
      `HTTP/MCP incident ${field} mismatch: http=${httpIncident[field]} mcp=${mcpIncident[field]}`,
    );
    assert(
      mcpResourceIncident[field] === httpIncident[field],
      `HTTP/MCP resource incident ${field} mismatch: http=${httpIncident[field]} mcp=${mcpResourceIncident[field]}`,
    );
  }

  await cleanup();
  console.log('Service incident HTTP/MCP parity smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
