#!/usr/bin/env node

import {
  appendPriorRecoveryAttempt,
  assert,
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

function assertTraceParity(httpTrace, mcpTrace, browserId) {
  assert(httpTrace.success === true, `HTTP service trace failed: ${JSON.stringify(httpTrace)}`);
  assert(mcpTrace.success === true, `MCP service_trace failed: ${JSON.stringify(mcpTrace)}`);
  assert(mcpTrace.tool === 'service_trace', 'MCP service_trace payload tool mismatch');
  assert(Array.isArray(httpTrace.data?.events), 'HTTP service trace missing events array');
  assert(Array.isArray(mcpTrace.data?.events), 'MCP service_trace missing events array');

  const httpBlocked = assertRecoveryBudgetBlockedEvents(httpTrace.data.events, {
    browserId,
    label: 'HTTP incident parity trace',
  });
  const mcpBlocked = assertRecoveryBudgetBlockedEvents(mcpTrace.data.events, {
    browserId,
    label: 'MCP incident parity trace',
  });
  assert(
    mcpBlocked.faultedEvent.id === httpBlocked.faultedEvent.id,
    `HTTP/MCP faulted trace event mismatch: http=${httpBlocked.faultedEvent.id} mcp=${mcpBlocked.faultedEvent.id}`,
  );
  assert(
    mcpTrace.data.counts?.events === mcpTrace.data.events.length,
    'MCP service_trace event count does not match returned events',
  );
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
  assertTraceParity(trace, mcpTrace, browserId);

  const httpIncidents = await httpJson(
    port,
    'GET',
    `/api/service/incidents?severity=critical&escalation=os_degraded_possible&service-name=${encodeURIComponent(
      serviceName,
    )}&agent-name=${encodeURIComponent(agentName)}&task-name=${encodeURIComponent(taskName)}&limit=20`,
  );
  const httpIncident = assertCriticalIncidentPayload(httpIncidents, 'HTTP', browserId);

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

  for (const field of ['id', 'severity', 'escalation', 'currentHealth', 'latestKind', 'recommendedAction']) {
    assert(
      mcpIncident[field] === httpIncident[field],
      `HTTP/MCP incident ${field} mismatch: http=${httpIncident[field]} mcp=${mcpIncident[field]}`,
    );
  }

  await cleanup();
  console.log('Service incident HTTP/MCP parity smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
