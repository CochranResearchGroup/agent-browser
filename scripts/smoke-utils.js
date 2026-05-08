import { spawn } from 'node:child_process';
import { request } from 'node:http';
import { createConnection } from 'node:net';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

export const rootDir = new URL('..', import.meta.url).pathname;

export function createSmokeContext({ prefix, session, sessionPrefix, socketDir: customSocketDir, socketSubdir = 's' }) {
  const tempHome = mkdtempSync(join(tmpdir(), prefix));
  const realHome = process.env.HOME;
  const cargoHome = process.env.CARGO_HOME || (realHome ? join(realHome, '.cargo') : undefined);
  const rustupHome = process.env.RUSTUP_HOME || (realHome ? join(realHome, '.rustup') : undefined);
  const smokeSession = session ?? `${sessionPrefix}-${process.pid}`;
  const agentHome = join(tempHome, '.agent-browser');
  const socketDir = customSocketDir
    ? customSocketDir({ agentHome, tempHome })
    : join(tempHome, socketSubdir);

  mkdirSync(socketDir, { recursive: true });

  const env = {
    ...process.env,
    HOME: tempHome,
    AGENT_BROWSER_HOME: agentHome,
    AGENT_BROWSER_SOCKET_DIR: socketDir,
    AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0',
    ...(cargoHome ? { CARGO_HOME: cargoHome } : {}),
    ...(rustupHome ? { RUSTUP_HOME: rustupHome } : {}),
  };

  return {
    agentHome,
    env,
    session: smokeSession,
    socketDir,
    tempHome,
    cleanupTempHome() {
      rmSync(tempHome, { recursive: true, force: true });
    },
  };
}

export function cargoArgs(args) {
  return ['run', '--quiet', '--manifest-path', 'cli/Cargo.toml', '--', ...args];
}

export function runCli(context, args, timeoutMs = 60000) {
  return new Promise((resolve, reject) => {
    const proc = spawn('cargo', cargoArgs(args), {
      cwd: rootDir,
      env: context.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const procTimeout = setTimeout(() => {
      proc.kill('SIGTERM');
      reject(new Error(`agent-browser ${args.join(' ')} timed out`));
    }, timeoutMs);
    let out = '';
    let err = '';
    proc.stdout.setEncoding('utf8');
    proc.stderr.setEncoding('utf8');
    proc.stdout.on('data', (chunk) => {
      out += chunk;
    });
    proc.stderr.on('data', (chunk) => {
      err += chunk;
    });
    proc.on('error', (err) => {
      clearTimeout(procTimeout);
      reject(err);
    });
    proc.on('exit', (code, signal) => {
      clearTimeout(procTimeout);
      if (code === 0) {
        resolve({ stdout: out, stderr: err });
      } else {
        reject(
          new Error(
            `agent-browser ${args.join(' ')} failed: code=${code} signal=${signal}\n${out}${err}`,
          ),
        );
      }
    });
  });
}

export function parseJsonOutput(output, label) {
  const text = output.trim();
  try {
    return JSON.parse(text);
  } catch (err) {
    throw new Error(`Failed to parse ${label} JSON output: ${err.message}\n${output}`);
  }
}

export function assertServiceStatusDidNotLaunch(status, label) {
  assert(
    status.data?.control_plane?.browser_health === 'NotStarted',
    `${label} launched browser: ${JSON.stringify(status.data?.control_plane)}`,
  );
}

export async function seedIncidentSummarySmokeState(context, {
  agentName,
  launchLabel = 'service status',
  serviceName,
  taskName,
}) {
  const result = await runCli(context, ['--json', '--session', context.session, 'service', 'status']);
  const status = parseJsonOutput(result.stdout, launchLabel);
  assert(status.success === true, `${launchLabel} failed before seed: ${result.stdout}${result.stderr}`);
  assertServiceStatusDidNotLaunch(status, `${launchLabel} before incident seed`);
  seedIncidentSummarySmokeEvents(context, { serviceName, agentName, taskName });
}

export function seedRemedyApplySmokeBrowser(context, {
  browserId,
  health,
  lastError,
}) {
  const serviceDir = join(context.agentHome, 'service');
  const statePath = join(serviceDir, 'state.json');
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  state.browsers[browserId] = {
    id: browserId,
    pid: null,
    cdpPort: null,
    cdpUrl: null,
    profileId: 'summary-profile',
    runtimeProfile: null,
    sessionId: context.session,
    activeSessionIds: [context.session],
    tabIds: [],
    health,
    lastError,
    lastHealthObservation: null,
    launchedAt: null,
    updatedAt: '2026-05-01T10:02:00Z',
  };
  writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`);
}

export function seedDegradedRemedySmokeBrowser(context) {
  seedRemedyApplySmokeBrowser(context, {
    browserId: 'browser-summary-degraded',
    health: 'degraded',
    lastError: 'Polite browser close failed; force kill was required',
  });
}

export function seedOsDegradedRemedySmokeBrowsers(context) {
  seedRemedyApplySmokeBrowser(context, {
    browserId: 'browser-summary-faulted-1',
    health: 'faulted',
    lastError: 'Force kill failed; host OS may be degraded',
  });
  seedRemedyApplySmokeBrowser(context, {
    browserId: 'browser-summary-faulted-2',
    health: 'faulted',
    lastError: 'Force kill failed; host OS may be degraded',
  });
}

export function seedIncidentSummarySmokeEvents(context, {
  agentName,
  serviceName,
  taskName,
}) {
  const serviceDir = join(context.agentHome, 'service');
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
      sessionId: context.session,
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
      sessionId: context.session,
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
      sessionId: context.session,
      serviceName,
      agentName,
      taskName,
      previousHealth: 'ready',
      currentHealth: 'degraded',
      details: { failureClass: 'polite_close_failed' },
    },
  ];
  writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`);
}

function findIncidentSummaryGroup(summary, escalation, severity, state) {
  return summary.groups.find(
    (group) =>
      group.escalation === escalation &&
      group.severity === severity &&
      group.state === state,
  );
}

export const remedySmokeExpectedGroups = [
  {
    escalation: 'os_degraded_possible',
    severity: 'critical',
    state: 'active',
    count: 2,
    incidentIds: ['browser-summary-faulted-1', 'browser-summary-faulted-2'],
    browserIds: ['browser-summary-faulted-1', 'browser-summary-faulted-2'],
    remedyApplyCommand: 'agent-browser service remedies apply --escalation os_degraded_possible',
    recommendedActionIncludes: 'host OS',
  },
  {
    escalation: 'browser_degraded',
    severity: 'warning',
    state: 'active',
    count: 1,
    incidentIds: ['browser-summary-degraded'],
    browserIds: ['browser-summary-degraded'],
    remedyApplyCommand: 'agent-browser service remedies apply --escalation browser_degraded',
    recommendedActionIncludes: 'browser health',
  },
];

export function expectedRemedySmokeResponse({ groupCount = 2 } = {}) {
  return {
    count: 3,
    matched: 3,
    groupCount,
    groups: remedySmokeExpectedGroups,
  };
}

export function incidentSummarySmokeFilterCases({
  agentName,
  serviceName,
  session,
  taskName,
}) {
  return [
    {
      label: 'compact remedy groups',
      httpQuery:
        `summary=true&remedies=true&limit=20&service-name=${encodeURIComponent(serviceName)}` +
        `&agent-name=${encodeURIComponent(agentName)}&task-name=${encodeURIComponent(taskName)}`,
      mcpArguments: {
        summary: true,
        remediesOnly: true,
        limit: 20,
        serviceName,
        agentName,
        taskName,
      },
      expected: {
        ...expectedRemedySmokeResponse(),
      },
    },
    {
      label: 'critical active unacknowledged filters',
      httpQuery:
        `summary=true&limit=20&state=active&severity=critical&escalation=os_degraded_possible` +
        `&handling-state=unacknowledged&profile-id=summary-profile&session-id=${encodeURIComponent(session)}` +
        `&service-name=${encodeURIComponent(serviceName)}&agent-name=${encodeURIComponent(agentName)}` +
        `&task-name=${encodeURIComponent(taskName)}`,
      mcpArguments: {
        summary: true,
        limit: 20,
        state: 'active',
        severity: 'critical',
        escalation: 'os_degraded_possible',
        handlingState: 'unacknowledged',
        profileId: 'summary-profile',
        sessionId: session,
        serviceName,
        agentName,
        taskName,
      },
      expected: {
        count: 2,
        matched: 2,
        groupCount: 1,
        groups: [
          {
            escalation: 'os_degraded_possible',
            severity: 'critical',
            state: 'active',
            count: 2,
            incidentIds: ['browser-summary-faulted-1', 'browser-summary-faulted-2'],
            browserIds: ['browser-summary-faulted-1', 'browser-summary-faulted-2'],
            remedyApplyCommand: 'agent-browser service remedies apply --escalation os_degraded_possible',
            recommendedActionIncludes: 'host OS',
          },
        ],
      },
    },
    {
      label: 'warning browser and since filters',
      httpQuery:
        `summary=true&limit=20&state=active&severity=warning&escalation=browser_degraded` +
        `&browser-id=browser-summary-degraded&since=2026-05-01T10%3A02%3A00Z` +
        `&service-name=${encodeURIComponent(serviceName)}&agent-name=${encodeURIComponent(agentName)}` +
        `&task-name=${encodeURIComponent(taskName)}`,
      mcpArguments: {
        summary: true,
        limit: 20,
        state: 'active',
        severity: 'warning',
        escalation: 'browser_degraded',
        browserId: 'browser-summary-degraded',
        since: '2026-05-01T10:02:00Z',
        serviceName,
        agentName,
        taskName,
      },
      expected: {
        count: 1,
        matched: 1,
        groupCount: 1,
        groups: [
          {
            escalation: 'browser_degraded',
            severity: 'warning',
            state: 'active',
            count: 1,
            incidentIds: ['browser-summary-degraded'],
            browserIds: ['browser-summary-degraded'],
            remedyApplyCommand: 'agent-browser service remedies apply --escalation browser_degraded',
            recommendedActionIncludes: 'browser health',
          },
        ],
      },
    },
    {
      label: 'nonmatching dashboard summary filters',
      httpQuery:
        `summary=true&limit=20&state=active&severity=critical&escalation=os_degraded_possible` +
        `&since=2026-05-01T10%3A02%3A00Z&service-name=${encodeURIComponent(serviceName)}` +
        `&agent-name=${encodeURIComponent(agentName)}&task-name=${encodeURIComponent(taskName)}`,
      mcpArguments: {
        summary: true,
        limit: 20,
        state: 'active',
        severity: 'critical',
        escalation: 'os_degraded_possible',
        since: '2026-05-01T10:02:00Z',
        serviceName,
        agentName,
        taskName,
      },
      expected: {
        count: 0,
        matched: 0,
        groupCount: 0,
        groups: [],
      },
    },
  ];
}

export function assertIncidentSummarySmokeShape(summary, label) {
  assert(summary && typeof summary === 'object', `${label} missing summary object: ${JSON.stringify(summary)}`);
  assert(Number.isInteger(summary.groupCount), `${label} summary missing groupCount: ${JSON.stringify(summary)}`);
  assert(Array.isArray(summary.groups), `${label} summary missing groups: ${JSON.stringify(summary)}`);
  assert(summary.groupCount === summary.groups.length, `${label} summary count mismatch: ${JSON.stringify(summary)}`);

  const critical = findIncidentSummaryGroup(summary, 'os_degraded_possible', 'critical', 'active');
  assert(critical, `${label} summary missing critical OS group: ${JSON.stringify(summary)}`);
  assert(critical.count === 2, `${label} summary critical count mismatch: ${JSON.stringify(critical)}`);
  assert(
    critical.incidentIds.includes('browser-summary-faulted-1') &&
      critical.incidentIds.includes('browser-summary-faulted-2'),
    `${label} summary critical IDs mismatch: ${JSON.stringify(critical)}`,
  );
  assert(
    critical.browserIds.includes('browser-summary-faulted-1') &&
      critical.browserIds.includes('browser-summary-faulted-2'),
    `${label} summary critical browser IDs mismatch: ${JSON.stringify(critical)}`,
  );
  assert(
    critical.remedyApplyCommand === 'agent-browser service remedies apply --escalation os_degraded_possible',
    `${label} summary critical apply command mismatch: ${JSON.stringify(critical)}`,
  );
  assert(
    critical.recommendedAction.includes('host OS'),
    `${label} summary critical remedy mismatch: ${JSON.stringify(critical)}`,
  );

  const warning = findIncidentSummaryGroup(summary, 'browser_degraded', 'warning', 'active');
  assert(warning, `${label} summary missing degraded-browser group: ${JSON.stringify(summary)}`);
  assert(warning.count === 1, `${label} summary warning count mismatch: ${JSON.stringify(warning)}`);
  assert(
    warning.incidentIds.includes('browser-summary-degraded'),
    `${label} summary warning IDs mismatch: ${JSON.stringify(warning)}`,
  );
  assert(
    warning.browserIds.includes('browser-summary-degraded'),
    `${label} summary warning browser IDs mismatch: ${JSON.stringify(warning)}`,
  );
  assert(
    warning.remedyApplyCommand === 'agent-browser service remedies apply --escalation browser_degraded',
    `${label} summary warning apply command mismatch: ${JSON.stringify(warning)}`,
  );
}

export function assertIncidentSummaryFilteredResponse(data, expected, label) {
  assert(data && typeof data === 'object', `${label} missing response data: ${JSON.stringify(data)}`);
  assert(data.count === expected.count, `${label} count mismatch: ${JSON.stringify(data)}`);
  assert(data.matched === expected.matched, `${label} matched mismatch: ${JSON.stringify(data)}`);
  assert(data.summary && typeof data.summary === 'object', `${label} missing summary: ${JSON.stringify(data)}`);
  assert(
    data.summary.groupCount === expected.groupCount,
    `${label} summary group count mismatch: ${JSON.stringify(data.summary)}`,
  );
  assert(
    Array.isArray(data.summary.groups),
    `${label} summary groups missing: ${JSON.stringify(data.summary)}`,
  );
  assert(
    data.summary.groups.length === expected.groupCount,
    `${label} summary groups length mismatch: ${JSON.stringify(data.summary)}`,
  );

  for (const expectedGroup of expected.groups) {
    const group = findIncidentSummaryGroup(
      data.summary,
      expectedGroup.escalation,
      expectedGroup.severity,
      expectedGroup.state,
    );
    assert(group, `${label} missing expected group ${JSON.stringify(expectedGroup)} in ${JSON.stringify(data.summary)}`);
    assert(group.count === expectedGroup.count, `${label} group count mismatch: ${JSON.stringify(group)}`);
    for (const incidentId of expectedGroup.incidentIds) {
      assert(
        group.incidentIds.includes(incidentId),
        `${label} missing incident ${incidentId} in ${JSON.stringify(group)}`,
      );
    }
    for (const browserId of expectedGroup.browserIds ?? []) {
      assert(
        group.browserIds.includes(browserId),
        `${label} missing browser ${browserId} in ${JSON.stringify(group)}`,
      );
    }
    if (expectedGroup.remedyApplyCommand) {
      assert(
        group.remedyApplyCommand === expectedGroup.remedyApplyCommand,
        `${label} remedy apply command mismatch: ${JSON.stringify(group)}`,
      );
    }
    if (expectedGroup.recommendedActionIncludes) {
      assert(
        group.recommendedAction.includes(expectedGroup.recommendedActionIncludes),
        `${label} recommended action mismatch: ${JSON.stringify(group)}`,
      );
    }
  }
}

export function assertServiceRemediesJsonResponse(data, label) {
  assert(data?.filters?.remediesOnly === true, `${label} missing remediesOnly filter: ${JSON.stringify(data)}`);
  assert(data?.filters?.state === 'active', `${label} missing active state filter: ${JSON.stringify(data)}`);
  assertIncidentSummaryFilteredResponse(data, expectedRemedySmokeResponse(), label);
}

export function assertServiceRemediesTextOutput(output, label) {
  assert(output.includes('Incident groups: 2'), `${label} missing group count:\n${output}`);
  for (const group of remedySmokeExpectedGroups) {
    assert(
      output.includes(
        `${group.severity} escalation=${group.escalation} state=${group.state} count=${group.count}`,
      ),
      `${label} missing ${group.escalation} group:\n${output}`,
    );
    assert(
      output.includes(`browsers=${group.browserIds.join(',')}`),
      `${label} missing ${group.escalation} browsers:\n${output}`,
    );
    assert(
      output.includes(`apply=${group.remedyApplyCommand}`),
      `${label} missing ${group.escalation} apply command:\n${output}`,
    );
  }
}

export function assertBrowserRemediesApplyJsonResponse(data, {
  browserIds,
  escalation,
  label,
}) {
  assert(data?.applied === true, `${label} missing applied=true: ${JSON.stringify(data)}`);
  assert(data.escalation === escalation, `${label} escalation mismatch: ${JSON.stringify(data)}`);
  assert(data.count === browserIds.length, `${label} count mismatch: ${JSON.stringify(data)}`);
  assert(Array.isArray(data.monitorIds), `${label} missing monitorIds array: ${JSON.stringify(data)}`);
  assert(data.monitorIds.length === 0, `${label} monitorIds should be empty: ${JSON.stringify(data)}`);
  assert(Array.isArray(data.monitorResults), `${label} missing monitorResults array: ${JSON.stringify(data)}`);
  assert(data.monitorResults.length === 0, `${label} monitorResults should be empty: ${JSON.stringify(data)}`);
  assert(Array.isArray(data.browserIds), `${label} missing browserIds array: ${JSON.stringify(data)}`);
  for (const browserId of browserIds) {
    assert(data.browserIds.includes(browserId), `${label} missing browser ID ${browserId}: ${JSON.stringify(data)}`);
  }
  assert(Array.isArray(data.browserResults), `${label} missing browserResults array: ${JSON.stringify(data)}`);
  assert(data.browserResults.length === browserIds.length, `${label} browserResults length mismatch: ${JSON.stringify(data)}`);

  for (const browserId of browserIds) {
    const result = data.browserResults.find((entry) => entry.id === browserId);
    assert(result, `${label} missing browser result ${browserId}: ${JSON.stringify(data)}`);
    assert(result.retryEnabled === true, `${label} browser retryEnabled mismatch: ${JSON.stringify(result)}`);
    assert(result.browser?.id === browserId, `${label} browser record ID mismatch: ${JSON.stringify(result)}`);
    assert(result.browser?.health === 'process_exited', `${label} browser health mismatch: ${JSON.stringify(result)}`);
    assert(
      result.browser?.lastHealthObservation?.failureClass === 'browser_process_exited',
      `${label} missing retry health observation: ${JSON.stringify(result)}`,
    );
    assert(result.incident?.browserId === browserId, `${label} incident browser mismatch: ${JSON.stringify(result)}`);
    assert(result.incident?.escalation === 'browser_recovery', `${label} incident escalation mismatch: ${JSON.stringify(result)}`);
  }
}

export function assertBrowserDegradedRemediesApplyJsonResponse(data, label) {
  assertBrowserRemediesApplyJsonResponse(data, {
    browserIds: ['browser-summary-degraded'],
    escalation: 'browser_degraded',
    label,
  });
}

export function assertOsDegradedRemediesApplyJsonResponse(data, label) {
  assertBrowserRemediesApplyJsonResponse(data, {
    browserIds: ['browser-summary-faulted-1', 'browser-summary-faulted-2'],
    escalation: 'os_degraded_possible',
    label,
  });
}

export function httpJson(port, method, path, body) {
  return new Promise((resolve, reject) => {
    const rawBody = body === undefined ? undefined : JSON.stringify(body);
    const req = request(
      {
        host: '127.0.0.1',
        port,
        method,
        path,
        headers: rawBody
          ? {
              'content-type': 'application/json',
              'content-length': Buffer.byteLength(rawBody),
            }
          : undefined,
      },
      (res) => {
        let text = '';
        res.setEncoding('utf8');
        res.on('data', (chunk) => {
          text += chunk;
        });
        res.on('end', () => {
          try {
            const parsed = JSON.parse(text);
            if (!res.statusCode || res.statusCode < 200 || res.statusCode >= 300) {
              reject(new Error(`HTTP ${method} ${path} returned ${res.statusCode}: ${text}`));
              return;
            }
            resolve(parsed);
          } catch (err) {
            reject(new Error(`Failed to parse HTTP ${method} ${path}: ${err.message}\n${text}`));
          }
        });
      },
    );
    req.setTimeout(30000, () => {
      req.destroy(new Error(`HTTP ${method} ${path} timed out`));
    });
    req.on('error', reject);
    if (rawBody) req.write(rawBody);
    req.end();
  });
}

export function httpJsonResult(port, method, path, body) {
  return new Promise((resolve, reject) => {
    const rawBody = body === undefined ? undefined : JSON.stringify(body);
    const req = request(
      {
        host: '127.0.0.1',
        port,
        method,
        path,
        headers: rawBody
          ? {
              'content-type': 'application/json',
              'content-length': Buffer.byteLength(rawBody),
            }
          : undefined,
      },
      (res) => {
        let text = '';
        res.setEncoding('utf8');
        res.on('data', (chunk) => {
          text += chunk;
        });
        res.on('end', () => {
          try {
            resolve({
              body: text ? JSON.parse(text) : undefined,
              statusCode: res.statusCode,
            });
          } catch (err) {
            reject(new Error(`Failed to parse HTTP ${method} ${path}: ${err.message}\n${text}`));
          }
        });
      },
    );
    req.setTimeout(30000, () => {
      req.destroy(new Error(`HTTP ${method} ${path} timed out`));
    });
    req.on('error', reject);
    if (rawBody) req.write(rawBody);
    req.end();
  });
}

export function assert(condition, message) {
  if (!condition) throw new Error(message);
}

export function recoveryOverrideSmokeUrls(label) {
  return {
    blockedUrl: smokeDataUrl(`Blocked ${label}`, `Blocked ${label}`),
    initialUrl: smokeDataUrl(label, label),
    recoveredUrl: smokeDataUrl(`Recovered ${label}`, `Recovered ${label}`),
  };
}

export function smokeDataUrl(title, heading) {
  const html = [
    '<!doctype html>',
    '<html>',
    `<head><title>${title}</title></head>`,
    `<body><h1 id="ready">${heading}</h1></body>`,
    '</html>',
  ].join('');
  return `data:text/html;charset=utf-8,${encodeURIComponent(html)}`;
}

export function seedServiceOwnershipHandoff(context, {
  browserId,
  handoffSession,
  legacySession,
  liveTabId,
  staleTabId,
  staleTargetId,
  staleTitle,
}) {
  const path = join(context.agentHome, 'service', 'state.json');
  const state = JSON.parse(readFileSync(path, 'utf8'));
  assert(state.browsers?.[browserId], `Cannot seed handoff; missing browser ${browserId}`);
  state.browsers[browserId].activeSessionIds = [handoffSession];
  state.sessions = {
    ...(state.sessions || {}),
    [legacySession]: {
      id: legacySession,
      serviceName: 'LegacyService',
      agentName: 'legacy-agent',
      taskName: 'staleOwner',
      lease: 'shared',
      cleanup: 'detach',
      browserIds: [browserId],
      tabIds: [liveTabId, staleTabId],
    },
  };
  state.tabs = {
    ...(state.tabs || {}),
    [liveTabId]: {
      ...(state.tabs?.[liveTabId] || {}),
      ownerSessionId: legacySession,
    },
    [staleTabId]: {
      id: staleTabId,
      browserId,
      targetId: staleTargetId,
      lifecycle: 'ready',
      ownerSessionId: legacySession,
      url: 'https://stale.example.invalid/',
      title: staleTitle,
    },
  };
  writeFileSync(path, `${JSON.stringify(state, null, 2)}\n`);
}

export function assertServiceOwnershipHandoff(collections, label, {
  browserId,
  handoffSession,
  legacySession,
  liveTabId,
  staleTabId,
}) {
  const liveTab = collections.tabs?.find((tab) => tab.id === liveTabId);
  const staleTab = collections.tabs?.find((tab) => tab.id === staleTabId);
  const newOwner = collections.sessions?.find((item) => item.id === handoffSession);
  const oldOwner = collections.sessions?.find((item) => item.id === legacySession);

  assert(liveTab, `${label} missing live tab ${liveTabId}: ${JSON.stringify(collections.tabs)}`);
  assert(staleTab, `${label} missing stale tab ${staleTabId}: ${JSON.stringify(collections.tabs)}`);
  assert(
    newOwner,
    `${label} missing handoff session ${handoffSession}: ${JSON.stringify(collections.sessions)}`,
  );
  assert(oldOwner, `${label} missing legacy session ${legacySession}: ${JSON.stringify(collections.sessions)}`);
  assert(liveTab.browserId === browserId, `${label} live tab browser mismatch: ${JSON.stringify(liveTab)}`);
  assert(liveTab.lifecycle === 'ready', `${label} live tab was not ready: ${JSON.stringify(liveTab)}`);
  assert(
    liveTab.ownerSessionId === handoffSession,
    `${label} live tab owner was not reassigned: ${JSON.stringify(liveTab)}`,
  );
  assert(staleTab.lifecycle === 'closed', `${label} stale tab was not closed: ${JSON.stringify(staleTab)}`);
  assert(
    newOwner.tabIds?.includes(liveTabId),
    `${label} handoff session did not receive live tab: ${JSON.stringify(newOwner)}`,
  );
  assert(
    !oldOwner.tabIds?.includes(liveTabId) && !oldOwner.tabIds?.includes(staleTabId),
    `${label} legacy session retained browser tabs: ${JSON.stringify(oldOwner)}`,
  );
}

export function assertServiceOwnershipRepairEvent(events, label, {
  browserId,
  handoffSession,
  legacySession,
  liveTabId,
  staleTabId,
}) {
  const repairEvent = events?.find(
    (event) =>
      event.kind === 'reconciliation' &&
      event.browserId === browserId &&
      event.details?.action === 'session_tab_ownership_repaired',
  );
  assert(repairEvent, `${label} missing ownership repair event: ${JSON.stringify(events)}`);
  assert(
    repairEvent.details?.ownerSessionId === handoffSession,
    `${label} repair event owner mismatch: ${JSON.stringify(repairEvent)}`,
  );
  assert(
    repairEvent.details?.liveTabIds?.includes(liveTabId),
    `${label} repair event missing live tab ${liveTabId}: ${JSON.stringify(repairEvent)}`,
  );
  const removedRelations = repairEvent.details?.removedRelations || [];
  assert(
    removedRelations.some((relation) => relation.sessionId === legacySession && relation.tabId === liveTabId),
    `${label} repair event missing legacy live-tab relation: ${JSON.stringify(repairEvent)}`,
  );
  assert(
    removedRelations.some((relation) => relation.sessionId === legacySession && relation.tabId === staleTabId),
    `${label} repair event missing legacy stale-tab relation: ${JSON.stringify(repairEvent)}`,
  );

  return repairEvent;
}

export function configureRecoveryOverrideSmokeContext(context) {
  context.env.AGENT_BROWSER_SERVICE_RECOVERY_RETRY_BUDGET = '1';
  context.env.AGENT_BROWSER_SERVICE_RECOVERY_BASE_BACKOFF_MS = '1';
  context.env.AGENT_BROWSER_SERVICE_RECOVERY_MAX_BACKOFF_MS = '1';
  return context;
}

export function parseMcpToolPayload(result, label = 'MCP tool') {
  const text = result.content?.[0]?.text;
  assert(typeof text === 'string', `${label} response missing text content`);
  return JSON.parse(text);
}

export function appendPriorRecoveryAttempt(context, {
  agentName,
  browserId,
  serviceName,
  taskName,
}) {
  const path = join(context.agentHome, 'service', 'state.json');
  const state = JSON.parse(readFileSync(path, 'utf8'));
  state.events.push({
    id: `event-smoke-prior-recovery-${Date.now()}`,
    timestamp: new Date().toISOString(),
    kind: 'browser_recovery_started',
    message: `Browser ${browserId} recovery started`,
    browserId,
    sessionId: context.session,
    serviceName,
    agentName,
    taskName,
    currentHealth: 'process_exited',
    details: {
      reasonKind: 'process_exited',
      reason: 'Synthetic prior recovery attempt for blocked recovery override smoke',
      attempt: 1,
      retryBudget: 1,
      retryBudgetExceeded: false,
      nextRetryDelayMs: 1,
      policySource: {
        retryBudget: 'env',
        baseBackoffMs: 'env',
        maxBackoffMs: 'env',
      },
    },
  });
  writeFileSync(path, `${JSON.stringify(state, null, 2)}\n`);
}

const RECOVERY_STALE_HEALTH_VALUES = new Set(['process_exited', 'cdp_disconnected']);
const RECOVERY_REASON_KIND_BY_HEALTH = {
  cdp_disconnected: 'cdp_disconnected',
  process_exited: 'process_exited',
};

function eventIndex(events, predicate, label) {
  const index = events.findIndex(predicate);
  assert(index >= 0, `${label} missing from trace events: ${JSON.stringify(events)}`);
  return index;
}

export function assertRecoveryTraceEvents(events, { browserId, label = 'Recovery' }) {
  assert(Array.isArray(events), `${label} trace missing events array`);

  const staleIndex = eventIndex(
    events,
    (event) =>
      event.kind === 'browser_health_changed' &&
      event.browserId === browserId &&
      RECOVERY_STALE_HEALTH_VALUES.has(event.currentHealth),
    `${label} stale browser health event`,
  );
  const staleHealth = events[staleIndex].currentHealth;
  assert(
    events[staleIndex].details?.currentReasonKind === RECOVERY_REASON_KIND_BY_HEALTH[staleHealth],
    `${label} stale health event did not include structured reason kind: ${JSON.stringify(events[staleIndex])}`,
  );
  assert(
    typeof events[staleIndex].details?.failureClass === 'string' &&
      events[staleIndex].details.failureClass.length > 0,
    `${label} stale health event did not include failure class: ${JSON.stringify(events[staleIndex])}`,
  );
  if (staleHealth === 'process_exited') {
    assert(
      events[staleIndex].details?.processExitCause === 'unexpected_process_exit',
      `${label} process-exited health event did not include stable exit cause: ${JSON.stringify(events[staleIndex])}`,
    );
    if (events[staleIndex].details?.processExitDetection === 'local_child_try_wait') {
      assert(
        Number.isInteger(events[staleIndex].details?.processExitPid),
        `${label} process-exited health event did not include local process exit PID: ${JSON.stringify(events[staleIndex])}`,
      );
    }
  }
  const recoveryIndex = eventIndex(
    events,
    (event) =>
      event.kind === 'browser_recovery_started' &&
      event.browserId === browserId &&
      event.currentHealth === staleHealth,
    `${label} browser recovery started event`,
  );
  const readyIndex = eventIndex(
    events,
    (event) =>
      event.kind === 'browser_health_changed' &&
      event.browserId === browserId &&
      event.currentHealth === 'ready',
    `${label} ready browser health event`,
  );

  assert(
    staleIndex < recoveryIndex && recoveryIndex < readyIndex,
    `${label} recovery events were not ordered stale -> recovery -> ready: ${JSON.stringify(events)}`,
  );
  assert(
    typeof events[recoveryIndex].details?.reason === 'string' &&
      events[recoveryIndex].details.reason.length > 0,
    `${label} recovery event did not include crash reason: ${JSON.stringify(events[recoveryIndex])}`,
  );
  assert(
    events[recoveryIndex].details?.reasonKind === RECOVERY_REASON_KIND_BY_HEALTH[staleHealth],
    `${label} recovery event did not include structured reason kind: ${JSON.stringify(events[recoveryIndex])}`,
  );
  assert(
    typeof events[recoveryIndex].details?.failureClass === 'string' &&
      events[recoveryIndex].details.failureClass.length > 0,
    `${label} recovery event did not include failure class: ${JSON.stringify(events[recoveryIndex])}`,
  );
  if (staleHealth === 'process_exited') {
    assert(
      events[recoveryIndex].details?.processExitCause === 'unexpected_process_exit',
      `${label} process-exited recovery event did not include stable exit cause: ${JSON.stringify(events[recoveryIndex])}`,
    );
    if (events[recoveryIndex].details?.processExitDetection === 'local_child_try_wait') {
      assert(
        Number.isInteger(events[recoveryIndex].details?.processExitPid),
        `${label} process-exited recovery event did not include local process exit PID: ${JSON.stringify(events[recoveryIndex])}`,
      );
    }
  }
  assert(
    Number.isInteger(events[recoveryIndex].details?.attempt) &&
      events[recoveryIndex].details.attempt >= 1,
    `${label} recovery event did not include retry attempt: ${JSON.stringify(events[recoveryIndex])}`,
  );
  assert(
    Number.isInteger(events[recoveryIndex].details?.retryBudget) &&
      events[recoveryIndex].details.retryBudget >= events[recoveryIndex].details.attempt,
    `${label} recovery event did not include retry budget: ${JSON.stringify(events[recoveryIndex])}`,
  );
  assert(
    events[recoveryIndex].details?.retryBudgetExceeded === false,
    `${label} recovery event unexpectedly exceeded retry budget: ${JSON.stringify(events[recoveryIndex])}`,
  );
  assert(
    events[recoveryIndex].details?.policySource &&
      typeof events[recoveryIndex].details.policySource.retryBudget === 'string' &&
      typeof events[recoveryIndex].details.policySource.baseBackoffMs === 'string' &&
      typeof events[recoveryIndex].details.policySource.maxBackoffMs === 'string',
    `${label} recovery event did not include policy source metadata: ${JSON.stringify(events[recoveryIndex])}`,
  );

  return {
    readyEvent: events[readyIndex],
    recoveryEvent: events[recoveryIndex],
    staleEvent: events[staleIndex],
  };
}

export function assertRecoveryBudgetBlockedEvents(events, { browserId, label = 'Recovery budget' }) {
  assert(Array.isArray(events), `${label} trace missing events array`);

  const faultedIndex = eventIndex(
    events,
    (event) =>
      event.kind === 'browser_health_changed' &&
      event.browserId === browserId &&
      event.currentHealth === 'faulted',
    `${label} faulted browser health event`,
  );
  const faultedEvent = events[faultedIndex];
  assert(
    typeof faultedEvent.details?.currentError === 'string' &&
      faultedEvent.details.currentError.includes('retry budget exceeded'),
    `${label} faulted event did not include retry budget failure: ${JSON.stringify(faultedEvent)}`,
  );

  return { faultedEvent, faultedIndex };
}

export function assertServiceTracePayload(payload, label = 'Service trace', { tool = undefined } = {}) {
  assert(payload.success === true, `${label} failed: ${JSON.stringify(payload)}`);
  if (tool !== undefined) {
    assert(payload.tool === tool, `${label} payload tool mismatch`);
  }
  assert(Array.isArray(payload.data?.events), `${label} missing events array`);
  assert(Array.isArray(payload.data?.jobs), `${label} missing jobs array`);
  for (const job of payload.data.jobs) {
    assert(Array.isArray(job.namingWarnings), `${label} job missing naming warnings array`);
    assert(typeof job.hasNamingWarning === 'boolean', `${label} job missing naming warning flag`);
  }
  assert(Array.isArray(payload.data?.incidents), `${label} missing incidents array`);
  assert(Array.isArray(payload.data?.activity), `${label} missing activity array`);
  assert(payload.data?.summary && typeof payload.data.summary === 'object', `${label} missing summary object`);
  assert(Array.isArray(payload.data.summary.contexts), `${label} summary missing contexts array`);
  assertProfileLeaseWaitSummary(payload.data.summary.profileLeaseWaits, `${label} profile lease wait summary`);
  assert(
    payload.data.summary.contextCount === payload.data.summary.contexts.length,
    `${label} summary context count does not match returned contexts`,
  );
  assert(
    Number.isInteger(payload.data.summary.namingWarningCount),
    `${label} summary missing naming warning count`,
  );
  assert(
    payload.data.counts?.events === payload.data.events.length,
    `${label} event count does not match returned events`,
  );
  return payload;
}

function assertProfileLeaseWaitSummary(summary, label) {
  assert(summary && typeof summary === 'object', `${label} missing`);
  assert(Number.isInteger(summary.count), `${label} missing count`);
  assert(Number.isInteger(summary.activeCount), `${label} missing active count`);
  assert(Number.isInteger(summary.completedCount), `${label} missing completed count`);
  assert(Array.isArray(summary.waits), `${label} missing waits array`);
  assert(summary.count === summary.waits.length, `${label} count does not match waits length`);
  assert(
    summary.activeCount + summary.completedCount === summary.count,
    `${label} active/completed counts do not match total`,
  );

  for (const [index, wait] of summary.waits.entries()) {
    const waitLabel = `${label} wait[${index}]`;
    assert(wait && typeof wait === 'object', `${waitLabel} missing`);
    assert(typeof wait.jobId === 'string' && wait.jobId.length > 0, `${waitLabel} missing jobId`);
    assert(wait.profileId === null || typeof wait.profileId === 'string', `${waitLabel} invalid profileId`);
    assert(typeof wait.outcome === 'string' && wait.outcome.length > 0, `${waitLabel} missing outcome`);
    assert(wait.startedAt === null || typeof wait.startedAt === 'string', `${waitLabel} invalid startedAt`);
    assert(wait.endedAt === null || typeof wait.endedAt === 'string', `${waitLabel} invalid endedAt`);
    assert(wait.waitedMs === null || Number.isInteger(wait.waitedMs), `${waitLabel} invalid waitedMs`);
    assert(wait.retryAfterMs === null || Number.isInteger(wait.retryAfterMs), `${waitLabel} invalid retryAfterMs`);
    assert(Array.isArray(wait.conflictSessionIds), `${waitLabel} missing conflictSessionIds`);
    assert(wait.serviceName === null || typeof wait.serviceName === 'string', `${waitLabel} invalid serviceName`);
    assert(wait.agentName === null || typeof wait.agentName === 'string', `${waitLabel} invalid agentName`);
    assert(wait.taskName === null || typeof wait.taskName === 'string', `${waitLabel} invalid taskName`);
  }
}

export function assertHttpMcpServiceTraceEventParity({
  assertEvent,
  httpTrace,
  label = 'Service trace',
  mcpTrace,
}) {
  assert(typeof assertEvent === 'function', `${label} trace parity missing event assertion`);
  assertServiceTracePayload(httpTrace, `HTTP ${label}`);
  assertServiceTracePayload(mcpTrace, `MCP ${label}`, { tool: 'service_trace' });

  const httpEvent = assertEvent(httpTrace.data.events, `HTTP ${label}`);
  const mcpEvent = assertEvent(mcpTrace.data.events, `MCP ${label}`);
  assert(httpEvent?.id, `HTTP ${label} did not return a matched event`);
  assert(mcpEvent?.id, `MCP ${label} did not return a matched event`);
  assert(
    mcpEvent.id === httpEvent.id,
    `HTTP/MCP trace event mismatch: http=${httpEvent.id} mcp=${mcpEvent.id}`,
  );
  assert(
    JSON.stringify(mcpTrace.data.summary.profileLeaseWaits) ===
      JSON.stringify(httpTrace.data.summary.profileLeaseWaits),
    `${label} HTTP/MCP profile lease wait summary mismatch: http=${JSON.stringify(httpTrace.data.summary.profileLeaseWaits)} mcp=${JSON.stringify(mcpTrace.data.summary.profileLeaseWaits)}`,
  );

  return { httpEvent, mcpEvent };
}

export function assertRecoveryOverrideEvents(events, { browserId, actor, label = 'Recovery override' }) {
  assert(Array.isArray(events), `${label} trace missing events array`);

  const overrideIndex = eventIndex(
    events,
    (event) => event.kind === 'browser_recovery_override' && event.browserId === browserId,
    `${label} browser recovery override event`,
  );
  const overrideEvent = events[overrideIndex];
  assert(
    overrideEvent.details?.action === 'retry_enabled',
    `${label} override event did not record retry_enabled action: ${JSON.stringify(overrideEvent)}`,
  );
  if (actor !== undefined) {
    assert(
      overrideEvent.details?.actor === actor,
      `${label} override actor was ${overrideEvent.details?.actor}`,
    );
  }

  return { overrideEvent, overrideIndex };
}

export function assertRecoveryAfterOverride(events, {
  browserId,
  label = 'Recovery after override',
  overrideIndex = -1,
}) {
  const retryHealthIndex = events.findIndex(
    (event) =>
      event.kind === 'browser_health_changed' &&
      event.browserId === browserId &&
      event.previousHealth === 'faulted' &&
      event.currentHealth === 'process_exited',
  );
  assert(
    retryHealthIndex >= 0,
    `Final filtered trace lost the retry health event after override index ${overrideIndex}: ${JSON.stringify(events)}`,
  );
  return assertRecoveryTraceEvents(events.slice(retryHealthIndex), { browserId, label });
}

export function readResourceContents(response, label) {
  assert(response.success === true, `${label} read failed: ${JSON.stringify(response)}`);
  const contents = response.data?.contents;
  assert(contents && typeof contents === 'object', `${label} resource missing contents`);
  return contents;
}

export function daemonEndpoint(context) {
  if (process.platform === 'win32') {
    const port = Number(readFileSync(join(context.socketDir, `${context.session}.port`), 'utf8').trim());
    return { port, host: '127.0.0.1' };
  }
  return { path: join(context.socketDir, `${context.session}.sock`) };
}

export function sendRawCommand(context, command) {
  return new Promise((resolve, reject) => {
    const token = readFileSync(join(context.socketDir, `${context.session}.token`), 'utf8').trim();
    const socket = createConnection(daemonEndpoint(context));
    let response = '';
    const procTimeout = setTimeout(() => {
      socket.destroy();
      reject(new Error(`raw daemon command ${command.action} timed out`));
    }, 30000);
    socket.setEncoding('utf8');
    socket.on('connect', () => {
      socket.write(`${JSON.stringify({ ...command, _agentBrowserAuthToken: token })}\n`);
    });
    socket.on('data', (chunk) => {
      response += chunk;
      if (response.includes('\n')) {
        clearTimeout(procTimeout);
        socket.end();
        resolve(JSON.parse(response.trim()));
      }
    });
    socket.on('error', (err) => {
      clearTimeout(procTimeout);
      reject(err);
    });
  });
}

export function createMcpStdioClient({ context, args, onFatal }) {
  const child = spawn('cargo', cargoArgs(args), {
    cwd: rootDir,
    env: context.env,
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  let stdout = '';
  let stderr = '';
  let nextId = 1;
  const pending = new Map();

  function fatal(message) {
    for (const { reject } of pending.values()) reject(new Error(message));
    pending.clear();
    void onFatal(message, stderr);
  }

  function handleLine(line) {
    let message;
    try {
      message = JSON.parse(line);
    } catch (err) {
      fatal(`MCP server emitted non-JSON stdout line: ${line}\n${err.message}`);
      return;
    }

    const pendingRequest = pending.get(message.id);
    if (!pendingRequest) {
      fatal(`Received unexpected MCP response id: ${message.id}`);
      return;
    }

    pending.delete(message.id);
    if (message.error) {
      pendingRequest.reject(
        new Error(`${pendingRequest.method} failed: ${JSON.stringify(message.error)}`),
      );
      return;
    }
    pendingRequest.resolve(message.result);
  }

  child.stdout.setEncoding('utf8');
  child.stdout.on('data', (chunk) => {
    stdout += chunk;
    let newline = stdout.indexOf('\n');
    while (newline >= 0) {
      const line = stdout.slice(0, newline).trim();
      stdout = stdout.slice(newline + 1);
      if (line) handleLine(line);
      newline = stdout.indexOf('\n');
    }
  });

  child.stderr.setEncoding('utf8');
  child.stderr.on('data', (chunk) => {
    stderr += chunk;
  });

  child.on('error', (err) => {
    fatal(`Failed to spawn MCP server: ${err.message}`);
  });

  child.on('exit', (code, signal) => {
    if (pending.size > 0) {
      fatal(`MCP server exited before all responses arrived: code=${code} signal=${signal}`);
    }
  });

  return {
    close() {
      child.stdin.end();
      child.kill('SIGTERM');
    },
    notify(method, params) {
      const notification = { jsonrpc: '2.0', method };
      if (params !== undefined) notification.params = params;
      child.stdin.write(`${JSON.stringify(notification)}\n`);
    },
    rejectPending(message) {
      for (const { reject } of pending.values()) reject(new Error(message));
      pending.clear();
    },
    send(method, params) {
      const id = nextId++;
      const request = { jsonrpc: '2.0', id, method };
      if (params !== undefined) request.params = params;
      const promise = new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject, method });
      });
      child.stdin.write(`${JSON.stringify(request)}\n`);
      return promise;
    },
    stderr() {
      return stderr;
    },
  };
}

export async function closeSession(context) {
  try {
    await runCli(context, ['--json', '--session', context.session, 'close']);
  } catch {
    // The smoke may fail before the daemon starts; cleanup must stay best-effort.
  }
}
