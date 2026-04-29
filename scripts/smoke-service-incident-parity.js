#!/usr/bin/env node

import { readFileSync } from 'node:fs';

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

const context = configureRecoveryOverrideSmokeContext(
  createSmokeContext({ prefix: 'ab-sip-', sessionPrefix: 'sip' }),
);

const { session } = context;
const serviceName = 'IncidentParitySmoke';
const agentName = 'smoke-agent';
const taskName = 'compareHttpMcpIncidents';
const traceFields = { serviceName, agentName, taskName };
const incidentRecordSchema = JSON.parse(
  readFileSync(
    new URL('../docs/dev/contracts/service-incident-record.v1.schema.json', import.meta.url),
    'utf8',
  ),
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

function parseMcpJsonResource(result, uri, label) {
  const content = result.contents?.[0];
  assert(
    content?.mimeType === 'application/json',
    `${label} content MIME mismatch: ${JSON.stringify(content)}`,
  );
  assert(content?.uri === uri, `${label} content URI mismatch: ${JSON.stringify(content)}`);
  assert(typeof content.text === 'string', `${label} content missing JSON text`);
  return JSON.parse(content.text);
}

function schemaEnum(property) {
  return incidentRecordSchema.properties[property].enum;
}

function currentHealthSchemaEnum() {
  return incidentRecordSchema.properties.currentHealth.oneOf[0].enum;
}

function assertIncidentSchemaRecord(incident, label) {
  assert(
    incident && typeof incident === 'object',
    `${label} incident is not an object: ${JSON.stringify(incident)}`,
  );
  for (const field of incidentRecordSchema.required) {
    assert(
      Object.hasOwn(incident, field),
      `${label} incident missing schema field ${field}: ${JSON.stringify(incident)}`,
    );
  }
  for (const field of [
    'browser_id',
    'recommended_action',
    'acknowledged_at',
    'acknowledged_by',
    'acknowledgement_note',
    'resolved_at',
    'resolved_by',
    'resolution_note',
    'latest_timestamp',
    'latest_message',
    'latest_kind',
    'current_health',
    'event_ids',
    'job_ids',
  ]) {
    assert(!Object.hasOwn(incident, field), `${label} incident leaked snake_case field ${field}`);
  }
  assert(schemaEnum('state').includes(incident.state), `${label} incident state is outside schema enum`);
  assert(schemaEnum('severity').includes(incident.severity), `${label} incident severity is outside schema enum`);
  assert(
    schemaEnum('escalation').includes(incident.escalation),
    `${label} incident escalation is outside schema enum`,
  );
  assert(
    incident.currentHealth === null || currentHealthSchemaEnum().includes(incident.currentHealth),
    `${label} incident currentHealth is outside schema enum: ${JSON.stringify(incident)}`,
  );
  assert(Array.isArray(incident.eventIds), `${label} incident missing eventIds array`);
  assert(Array.isArray(incident.jobIds), `${label} incident missing jobIds array`);
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
  assertIncidentSchemaRecord(incident, label);
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
  assertHttpMcpServiceTraceEventParity({
    httpTrace: trace,
    mcpTrace,
    label: 'incident parity trace',
    assertEvent: (events, label) =>
      assertRecoveryBudgetBlockedEvents(events, { browserId, label }).faultedEvent,
  });

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
  assertIncidentSchemaRecord(httpIncidentDetail.data.incident, 'HTTP detail');

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
  assertIncidentSchemaRecord(mcpResourceIncident, 'MCP resource');

  const traceIncident = trace.data.incidents.find((item) => item.id === browserId);
  assert(
    traceIncident,
    `HTTP trace missing schema incident ${browserId}: ${JSON.stringify(trace.data.incidents)}`,
  );
  assertIncidentSchemaRecord(traceIncident, 'HTTP trace');
  const mcpTraceIncident = mcpTrace.data.incidents.find((item) => item.id === browserId);
  assert(
    mcpTraceIncident,
    `MCP trace missing schema incident ${browserId}: ${JSON.stringify(mcpTrace.data.incidents)}`,
  );
  assertIncidentSchemaRecord(mcpTraceIncident, 'MCP trace');

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
