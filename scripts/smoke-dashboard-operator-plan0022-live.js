#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { join } from 'node:path';

const args = process.argv.slice(2);
const options = {
  dashboardUrl: process.env.AGENT_BROWSER_DASHBOARD_URL || 'http://127.0.0.1:4848/',
  authEnv: process.env.AGENT_BROWSER_DASHBOARD_AUTH_ENV || `${homedir()}/.agent-browser/dashboard-auth.env`,
  json: false,
  runLiveOperation: false,
  runUiFollowup: false,
  requireLiveOperation: false,
  requireUiFollowup: false,
  agentBrowserBin: process.env.AGENT_BROWSER_BIN || 'agent-browser',
  browserProfile: '',
  uiSession: `plan0022-ui-followup-${process.pid}`,
};

for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === '--') {
    continue;
  } else if (arg === '--dashboard-url') {
    options.dashboardUrl = requiredValue(args, ++index, arg);
  } else if (arg === '--auth-env') {
    options.authEnv = requiredValue(args, ++index, arg);
  } else if (arg === '--json') {
    options.json = true;
  } else if (arg === '--run-live-operation') {
    options.runLiveOperation = true;
  } else if (arg === '--run-ui-followup') {
    options.runUiFollowup = true;
  } else if (arg === '--require-live-operation') {
    options.runLiveOperation = true;
    options.requireLiveOperation = true;
  } else if (arg === '--require-ui-followup') {
    options.runUiFollowup = true;
    options.requireUiFollowup = true;
  } else if (arg === '--agent-browser-bin') {
    options.agentBrowserBin = requiredValue(args, ++index, arg);
  } else if (arg === '--browser-profile') {
    options.browserProfile = requiredValue(args, ++index, arg);
  } else if (arg === '--ui-session') {
    options.uiSession = requiredValue(args, ++index, arg);
  } else if (arg === '--help' || arg === '-h') {
    printHelp();
    process.exit(0);
  } else {
    fail(`Unknown argument: ${arg}`);
  }
}

const report = {
  dashboardUrl: options.dashboardUrl,
  auth: null,
  operator: null,
  confirmation: null,
  liveOperation: null,
  uiFollowup: null,
};

try {
  await run();
  if (options.json) {
    console.log(JSON.stringify({ success: true, ...report }, null, 2));
  } else {
    console.log('Plan 0022 dashboard operator live smoke passed');
  }
} catch (err) {
  if (options.json) {
    console.log(JSON.stringify({
      success: false,
      error: err instanceof Error ? err.message : String(err),
      ...report,
    }, null, 2));
  } else {
    console.error(err instanceof Error ? err.stack || err.message : String(err));
  }
  process.exit(1);
}

async function run() {
  const base = new URL(options.dashboardUrl);
  const credentials = dashboardCredentials();
  const admin = await login(base, credentials.admin, 'admin superuser');
  const observer = await login(base, credentials.observer, 'observer');

  const unauthStatus = await requestJson(base, '/api/app-intelligence/operator/status');
  assertDeniedWithoutTools(unauthStatus, 'unauthenticated operator status');

  const observerStatus = await requestJson(base, '/api/app-intelligence/operator/status', {
    cookie: observer.cookie,
  });
  assertDeniedWithoutTools(observerStatus, 'observer operator status');

  const observerTurn = await requestJson(base, '/api/app-intelligence/operator/turn', {
    method: 'POST',
    cookie: observer.cookie,
    body: { prompt: 'Open a new browser to https://example.com', packet: fixturePacket() },
  });
  assertDeniedWithoutTools(observerTurn, 'observer operator turn');

  const adminStatus = await requestJson(base, '/api/app-intelligence/operator/status', {
    cookie: admin.cookie,
  });
  assert(
    adminStatus.response.ok && adminStatus.body?.success === true,
    `admin operator status failed: ${describeResponse(adminStatus)}`,
  );
  const groups = adminStatus.body?.data?.toolGroups ?? [];
  assert(Array.isArray(groups) && groups.length >= 5, `admin operator status missing tool groups: ${describeResponse(adminStatus)}`);
  assert(
    JSON.stringify(groups).includes('service_request:navigate') &&
      JSON.stringify(groups).includes('service_request:storage_clear'),
    `admin operator tools missing browser/service actions: ${JSON.stringify(groups)}`,
  );
  report.auth = {
    unauthStatus: unauthStatus.response.status,
    observerStatus: observerStatus.response.status,
    observerTurn: observerTurn.response.status,
    adminStatus: adminStatus.response.status,
    adminRole: adminStatus.body?.data?.authenticatedUser?.role ?? null,
  };

  const turn = await operatorTurn(base, admin.cookie, 'Clear session storage and clear cookies for the selected browser', fixturePacket());
  const actions = turn.data.dashboardActions ?? [];
  const toolCalls = turn.data.toolCalls ?? [];
  assert(
    toolCalls.some((call) => call.tool === 'propose_clear_storage' && call.status === 'proposed') &&
      toolCalls.some((call) => call.tool === 'propose_clear_cookies' && call.status === 'proposed'),
    `operator turn did not propose cleanup tools: ${JSON.stringify(toolCalls)}`,
  );
  const storageAction = findRequestAction(actions, 'storage_clear');
  const cookiesAction = findRequestAction(actions, 'cookies_clear');
  assertConfirmation(storageAction, 'storage_clear');
  assertConfirmation(cookiesAction, 'cookies_clear');
  assert(
    storageAction.request?.params?.scope === 'selected-tab-origin' &&
      storageAction.request?.params?.origin === 'http://127.0.0.1:38409',
    `storage_clear confirmation missing selected-origin scope: ${JSON.stringify(storageAction)}`,
  );
  assert(
    cookiesAction.request?.params?.scope === 'selected-browser-profile' &&
      String(cookiesAction.risk ?? '').includes('beyond the currently visible origin'),
    `cookies_clear confirmation missing profile scope warning: ${JSON.stringify(cookiesAction)}`,
  );
  report.operator = {
    runId: turn.data.runId,
    toolCalls: toolCalls.length,
    dashboardActions: actions.length,
  };
  report.confirmation = {
    storage: summarizeAction(storageAction),
    cookies: summarizeAction(cookiesAction),
  };

  if (options.runLiveOperation) {
    report.liveOperation = await runLiveBrowserOperation(base, admin.cookie);
    if (!report.liveOperation.success && options.requireLiveOperation) {
      throw new Error(`live browser operation did not complete: ${report.liveOperation.error || report.liveOperation.skippedReason}`);
    }
  }
  if (options.runUiFollowup) {
    report.uiFollowup = await runUiFollowup(base, credentials.admin);
    if (!report.uiFollowup.success && options.requireUiFollowup) {
      throw new Error(`UI follow-up smoke did not complete: ${report.uiFollowup.error || report.uiFollowup.skippedReason}`);
    }
  }
}

async function runLiveBrowserOperation(base, cookie) {
  const server = await startFixtureServer();
  try {
    const targetUrl = `http://127.0.0.1:${server.port}/`;
    const launchTurn = await operatorTurn(
      base,
      cookie,
      `Open a new browser to ${targetUrl}`,
      fixturePacket({ controllable: false, sessionId: null, url: 'about:blank' }),
    );
    const launchAction = findRequestAction(launchTurn.data.dashboardActions ?? [], 'tab_new');
    if (!launchAction) {
      return { success: false, skippedReason: 'operator did not return tab_new launch action', launchTurn: compactTurn(launchTurn.data) };
    }
    const launchResult = await serviceRequest(base, cookie, launchAction.request);
    if (!launchResult.response.ok || launchResult.body?.success === false) {
      return {
        success: false,
        skippedReason: 'launch service request unavailable',
        status: launchResult.response.status,
        error: launchResult.body?.error ?? null,
        body: launchResult.body,
      };
    }
    const launchPayload = launchResult.body;
    const launchedPacket = packetFromServiceResult(launchAction, launchPayload, targetUrl);
    const selection = launchedPacket.selection;

    const waitTurn = await operatorTurn(base, cookie, 'Wait for #operator-input', launchedPacket);
    await applyFirstServiceAction(base, cookie, waitTurn.data.dashboardActions, 'wait');

    const typeTurn = await operatorTurn(base, cookie, 'Type "Plan 0022" into #operator-input', launchedPacket);
    await applyFirstServiceAction(base, cookie, typeTurn.data.dashboardActions, 'type');

    const clickTurn = await operatorTurn(base, cookie, 'Click #operator-button', launchedPacket);
    await applyFirstServiceAction(base, cookie, clickTurn.data.dashboardActions, 'click');

    const snapshotTurn = await operatorTurn(base, cookie, 'Snapshot the DOM', launchedPacket);
    const snapshotResult = await applyFirstServiceAction(base, cookie, snapshotTurn.data.dashboardActions, 'snapshot');
    const snapshotText = JSON.stringify(snapshotResult.body?.data ?? snapshotResult.body ?? {});
    assert(
      snapshotText.includes('Applied Plan 0022'),
      `snapshot did not include operated page result: ${snapshotText.slice(0, 1000)}`,
    );

    return {
      success: true,
      targetUrl,
      selection,
      launch: summarizeServiceResponse(launchPayload),
      viewportSelectionDerivable: Boolean(selection.workspaceId || selection.browserId || selection.sessionId),
      operations: ['wait', 'type', 'click', 'snapshot'],
    };
  } catch (err) {
    return {
      success: false,
      error: err instanceof Error ? err.message : String(err),
    };
  } finally {
    await server.close();
  }
}

async function applyFirstServiceAction(base, cookie, actions, requestAction) {
  const action = findRequestAction(actions ?? [], requestAction);
  assert(action, `operator did not return ${requestAction}: ${JSON.stringify(actions)}`);
  assert(action.kind === 'service_request', `${requestAction} should be a direct service request in live smoke: ${JSON.stringify(action)}`);
  const result = await serviceRequest(base, cookie, action.request);
  assert(
    result.response.ok && result.body?.success !== false,
    `${requestAction} service request failed: ${describeResponse(result)}`,
  );
  return result;
}

async function operatorTurn(base, cookie, prompt, packet) {
  const response = await requestJson(base, '/api/app-intelligence/operator/turn', {
    method: 'POST',
    cookie,
    body: { prompt, packet },
  });
  assert(
    response.response.ok && response.body?.success === true && response.body?.data,
    `operator turn failed for ${JSON.stringify(prompt)}: ${describeResponse(response)}`,
  );
  return response.body;
}

async function serviceRequest(base, cookie, request) {
  return requestJson(base, '/api/service/request', {
    method: 'POST',
    cookie,
    body: request,
  });
}

function packetFromServiceResult(action, payload, targetUrl) {
  const data = payload?.data ?? payload ?? {};
  const params = action.request?.params ?? {};
  const browser = recordValue(data, 'browser');
  const session = recordValue(data, 'session');
  const tab = recordValue(data, 'tab');
  const profile = recordValue(data, 'profile');
  const browserId = firstString(
    recordValue(data, 'browserId'),
    recordValue(data, 'browser_id'),
    recordValue(browser, 'id'),
    recordValue(params, 'browserId'),
  );
  const sessionId = firstString(
    recordValue(data, 'sessionId'),
    recordValue(data, 'session_id'),
    recordValue(session, 'id'),
    browserId?.startsWith('session:') ? browserId.slice('session:'.length) : null,
  );
  const tabId = firstString(recordValue(data, 'tabId'), recordValue(data, 'targetId'), recordValue(tab, 'id'));
  const profileId = firstString(recordValue(data, 'profileId'), recordValue(data, 'runtimeProfile'), recordValue(profile, 'id'));
  return fixturePacket({
    browserId,
    controllable: true,
    profileId,
    sessionId,
    tabId,
    url: targetUrl,
    workspaceId: browserId ? `browser:${browserId}` : sessionId ? `browser:session:${sessionId}` : 'browser:session:operator-live',
  });
}

function fixturePacket({
  browserId = 'session:default',
  controllable = true,
  profileId = 'default',
  sessionId = 'default',
  tabId = 'target:abc',
  url = 'http://127.0.0.1:38409/app',
  workspaceId = 'browser:session:default',
} = {}) {
  return {
    version: 'selected-workspace-chat.v1',
    createdAt: new Date().toISOString(),
    provider: 'codex-app-server',
    selection: { workspaceId, browserId, sessionId, tabId, profileId, jobId: null },
    workspace: {
      id: workspaceId,
      label: sessionId || browserId || 'operator-live',
      source: 'attached_existing',
      state: 'active',
      health: 'ready',
      live: true,
      retained: false,
      viewable: true,
      controllable,
      missingReason: null,
    },
    runtime: {
      pid: 123,
      running: true,
      rssBytes: 10,
      cpuSeconds: 1,
      cdpPort: 9222,
      streamPort: 38395,
      lastFrameAt: 1780240000,
    },
    page: {
      title: 'Operator Smoke',
      url,
      targetId: tabId,
      lifecycle: 'active',
      active: true,
    },
    stream: {
      provider: 'cdp_screencast',
      routeSummary: 'cdp screencast',
      controlInput: 'cdp input',
      embeddable: true,
      controllable,
    },
    ownership: { serviceName: null, agentName: null, taskName: null },
    evidence: [
      { id: 'workspace.summary', source: 'workspace', summary: 'ready', facts: {}, freshness: 'fresh', included: true },
      { id: 'activity.unavailable', source: 'activity', summary: 'unavailable', facts: {}, freshness: 'unavailable', included: false },
      { id: 'console.unavailable', source: 'console', summary: 'unavailable', facts: {}, freshness: 'unavailable', included: false },
      { id: 'network.unavailable', source: 'network', summary: 'unavailable', facts: {}, freshness: 'unavailable', included: false },
      { id: 'storage.unavailable', source: 'storage', summary: 'unavailable', facts: {}, freshness: 'unavailable', included: false },
      { id: 'extensions.unavailable', source: 'extensions', summary: 'unavailable', facts: {}, freshness: 'unavailable', included: false },
    ],
    redaction: {
      secretsOmitted: true,
      screenshotsIncluded: false,
      rawStorageIncluded: false,
      rawHeadersIncluded: false,
    },
  };
}

async function startFixtureServer() {
  const server = createServer((req, res) => {
    if (req.url === '/favicon.ico') {
      res.writeHead(404).end();
      return;
    }
    res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    res.end(`<!doctype html>
<html>
  <head><title>Plan 0022 Operator Smoke</title></head>
  <body>
    <h1>Plan 0022 Operator Smoke</h1>
    <input id="operator-input" aria-label="Operator input" value="">
    <button id="operator-button" onclick="this.textContent = 'Applied ' + document.getElementById('operator-input').value; document.getElementById('operator-result').textContent = document.getElementById('operator-input').value + ' clicked'">Apply</button>
    <p id="operator-result">Waiting</p>
  </body>
</html>`);
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  return {
    port: server.address().port,
    close: () => new Promise((resolve) => server.close(resolve)),
  };
}

async function runUiFollowup(base, adminCredentials) {
  const server = await startFixtureServer();
  const generatedProfile = options.browserProfile ? '' : mkdtempSync(join(tmpdir(), 'ab-plan0022-ui-profile-'));
  const profile = options.browserProfile || generatedProfile;
  try {
    const targetUrl = `http://127.0.0.1:${server.port}/`;
    const dashboardUrl = dashboardWorkspaceSmokeUrl(base.href);
    await openDashboardUrl(dashboardUrl, profile);
    await runAgent(['--json', '--session', options.uiSession, 'wait', '1500'], { timeoutMs: 30000 });
    await evalAgent(`
(async () => {
  await fetch('/api/dashboard-auth/login', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    credentials: 'same-origin',
    body: JSON.stringify(${JSON.stringify(adminCredentials)})
  });
  window.localStorage.setItem('agent-browser-dashboard-right-pane-collapsed', 'false');
  location.href = ${JSON.stringify(dashboardUrl)};
  return JSON.stringify({ loginSubmitted: true, dashboardUrl: location.href });
})()
`, profile);
    await runAgent(['--json', '--session', options.uiSession, 'wait', '2000'], { timeoutMs: 30000 });
    await pollEval(profile, `
(() => {
  const clickByText = (text) => {
    const element = Array.from(document.querySelectorAll('button,[role=tab]'))
      .find((candidate) => candidate.textContent?.trim().includes(text));
    if (!element) return false;
    for (const type of ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'click']) {
      element.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, view: window }));
    }
    return true;
  };
  const chatted = clickByText('Chat');
  const operated = clickByText('Operate');
  const textarea = document.querySelector('textarea');
  return JSON.stringify({
    chatted,
    operated,
    hasTextarea: Boolean(textarea),
    url: location.href,
    text: document.body.innerText.slice(0, 800)
  });
})()
`, (state) => state.hasTextarea && state.operated !== false, 'UI Chat Operate editor');
    const submitted = await evalJson(`
(async () => {
  const clickByText = (text) => {
    const element = Array.from(document.querySelectorAll('button,[role=tab]'))
      .find((candidate) => candidate.textContent?.trim().includes(text));
    if (!element) return false;
    for (const type of ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'click']) {
      element.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, view: window }));
    }
    return true;
  };
  clickByText('Chat');
  await new Promise((resolve) => setTimeout(resolve, 500));
  const operated = clickByText('Operate');
  await new Promise((resolve) => setTimeout(resolve, 500));
  const textarea = document.querySelector('textarea');
  if (!textarea) return JSON.stringify({ operated, submitted: false, reason: 'missing textarea', text: document.body.innerText.slice(0, 500) });
  const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')?.set;
  setter?.call(textarea, ${JSON.stringify(`Open a new browser to ${targetUrl}`)});
  textarea.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: 'x' }));
  const form = textarea.closest('form');
  form?.requestSubmit();
  return JSON.stringify({ operated, submitted: Boolean(form), url: location.href });
})()
`, profile, 'UI operator prompt submit');
    assert(submitted.operated && submitted.submitted, `UI did not submit operator launch prompt: ${JSON.stringify(submitted)}`);

    const actionReady = await pollEval(profile, `
(() => {
  const text = document.body.innerText;
  const launchButton = Array.from(document.querySelectorAll('button'))
    .find((button) => button.textContent?.includes('Launch browser workspace'));
  return JSON.stringify({
    hasLaunchButton: Boolean(launchButton),
    hasOperator: text.includes('Superuser operator'),
    hasToolCall: text.includes('propose_launch_browser'),
    text: text.slice(0, 800)
  });
})()
`, (state) => state.hasLaunchButton, 'UI launch action');
    assert(actionReady.hasLaunchButton, `UI launch action did not appear: ${JSON.stringify(actionReady)}`);

    const launched = await evalJson(`
(async () => {
  const launchButton = Array.from(document.querySelectorAll('button'))
    .find((button) => button.textContent?.includes('Launch browser workspace'));
  if (!launchButton) return JSON.stringify({ clicked: false, reason: 'missing launch button' });
  for (const type of ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'click']) {
    launchButton.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, view: window }));
  }
  return JSON.stringify({ clicked: true });
})()
`, profile, 'UI launch action click');
    assert(launched.clicked, `UI did not click launch action: ${JSON.stringify(launched)}`);

    const followupReady = await pollEval(profile, `
(() => {
  const text = document.body.innerText;
  const followup = Array.from(document.querySelectorAll('button'))
    .find((button) => button.textContent?.includes('View launched browser'));
  return JSON.stringify({
    hasFollowup: Boolean(followup),
    hasServiceActivity: text.includes('operator.service_request') || text.includes('View launched browser'),
    url: location.href,
    text: text.slice(0, 1000)
  });
})()
`, (state) => state.hasFollowup, 'UI View launched browser follow-up', 90);
    assert(followupReady.hasFollowup, `UI follow-up did not appear: ${JSON.stringify(followupReady)}`);

    const beforeUrl = followupReady.url;
    const switched = await evalJson(`
(async () => {
  const followup = Array.from(document.querySelectorAll('button'))
    .find((button) => button.textContent?.includes('View launched browser'));
  if (!followup) return JSON.stringify({ clicked: false, reason: 'missing follow-up' });
  for (const type of ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'click']) {
    followup.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, view: window }));
  }
  await new Promise((resolve) => setTimeout(resolve, 500));
  return JSON.stringify({
    clicked: true,
    url: location.href,
    search: location.search,
    hasWorkspaceView: location.search.includes('view=workspace%3A') || location.search.includes('view=workspace:'),
    hasBrowserSelection: location.search.includes('browser=') || location.search.includes('session=') || location.search.includes('workspace=')
  });
})()
`, profile, 'UI View launched browser click');
    assert(
      switched.clicked && switched.hasBrowserSelection && switched.url !== beforeUrl,
      `UI did not switch dashboard URL selection: before=${beforeUrl} after=${JSON.stringify(switched)}`,
    );
    return {
      success: true,
      targetUrl,
      beforeUrl,
      afterUrl: switched.url,
      hasWorkspaceView: switched.hasWorkspaceView,
      hasBrowserSelection: switched.hasBrowserSelection,
    };
  } catch (err) {
    return {
      success: false,
      error: err instanceof Error ? err.message : String(err),
    };
  } finally {
    await runAgent(['--json', '--session', options.uiSession, 'close'], { timeoutMs: 30000 }).catch(() => undefined);
    await server.close();
    if (generatedProfile) {
      rmSync(generatedProfile, { recursive: true, force: true });
    }
  }
}

async function openDashboardUrl(url, profile) {
  const args = baseAgentArgs(profile);
  args.push('open', url);
  try {
    await runAgent(args, { timeoutMs: 90000 });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    if (!message.includes('Operation timed out. The page may still be loading')) {
      throw err;
    }
  }
}

function dashboardWorkspaceSmokeUrl(rawUrl) {
  const url = new URL(rawUrl);
  url.searchParams.set('view', 'workspace:control');
  url.searchParams.set('workspace', 'browser:session:default');
  url.searchParams.set('browser', 'session:default');
  url.searchParams.set('session', 'default');
  url.searchParams.set('profile', 'default');
  return url.href;
}

async function pollEval(profile, script, predicate, label, attempts = 75) {
  let latest = null;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    latest = await evalJson(script, profile, `${label} poll`);
    if (predicate(latest)) return latest;
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  throw new Error(`${label} did not become ready: ${JSON.stringify(latest)}`);
}

async function evalJson(script, profile, label) {
  const value = await evalAgent(script, profile);
  if (typeof value !== 'string') {
    throw new Error(`${label} did not return a JSON string: ${JSON.stringify(value)}`);
  }
  try {
    return JSON.parse(value);
  } catch (err) {
    throw new Error(`Failed to parse ${label} JSON: ${err instanceof Error ? err.message : String(err)}\n${value}`);
  }
}

async function evalAgent(script, profile) {
  const args = baseAgentArgs(profile);
  args.push('eval', '--stdin');
  const result = await runAgent(args, { input: script, timeoutMs: 60000 });
  const parsed = JSON.parse(result.stdout.trim());
  if (!parsed.success) {
    throw new Error(`agent-browser eval failed: ${result.stdout}${result.stderr}`);
  }
  return parsed.data?.result;
}

function baseAgentArgs(profile) {
  const args = ['--json', '--session', options.uiSession, '--browser-host', 'local_headless'];
  if (profile) {
    args.push('--profile', profile);
  }
  return args;
}

function runAgent(commandArgs, { input = '', timeoutMs = 60000 } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(options.agentBrowserBin, commandArgs, {
      cwd: new URL('..', import.meta.url),
      env: process.env,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    const timeout = setTimeout(() => {
      child.kill('SIGTERM');
      reject(new Error(`agent-browser command timed out: ${commandArgs.join(' ')}`));
    }, timeoutMs);
    let stdout = '';
    let stderr = '';
    child.stdout.setEncoding('utf8');
    child.stderr.setEncoding('utf8');
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
    });
    child.on('error', (err) => {
      clearTimeout(timeout);
      reject(err);
    });
    child.on('exit', (code, signal) => {
      clearTimeout(timeout);
      if (code === 0) {
        resolve({ stdout, stderr });
      } else {
        reject(new Error(`agent-browser ${commandArgs.join(' ')} failed with code=${code} signal=${signal}\n${stdout}${stderr}`));
      }
    });
    child.stdin.end(input);
  });
}

async function login(base, credentials, label) {
  const response = await requestJson(base, '/api/dashboard-auth/login', {
    method: 'POST',
    body: credentials,
  });
  const cookie = response.response.headers.get('set-cookie')?.split(';')[0] ?? '';
  assert(response.response.ok && cookie, `${label} login failed: ${describeResponse(response)}`);
  return { cookie, response };
}

async function requestJson(base, path, { method = 'GET', cookie = '', body } = {}) {
  const response = await fetch(new URL(path, base), {
    method,
    headers: {
      ...(body ? { 'content-type': 'application/json' } : {}),
      ...(cookie ? { cookie } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
    redirect: 'manual',
  });
  let parsed = null;
  const text = await response.text();
  if (text.trim()) {
    try {
      parsed = JSON.parse(text);
    } catch {
      parsed = { raw: text };
    }
  }
  return { response, body: parsed, text };
}

function dashboardCredentials() {
  if (!existsSync(options.authEnv)) {
    throw new Error(`Dashboard auth env file is missing: ${options.authEnv}`);
  }
  const values = parseEnv(readFileSync(options.authEnv, 'utf8'));
  const admin = {
    username: values.AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME || 'admin',
    password: values.AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD,
  };
  const observer = {
    username: values.AGENT_BROWSER_DASHBOARD_CODEX_USERNAME || 'codex',
    password: values.AGENT_BROWSER_DASHBOARD_CODEX_PASSWORD,
  };
  if (!admin.password) throw new Error(`Missing admin password in ${options.authEnv}`);
  if (!observer.password) throw new Error(`Missing observer/codex password in ${options.authEnv}`);
  return { admin, observer };
}

function parseEnv(text) {
  const values = {};
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const index = trimmed.indexOf('=');
    if (index <= 0) continue;
    const key = trimmed.slice(0, index).trim();
    let value = trimmed.slice(index + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    values[key] = value.replace(/\\"/g, '"');
  }
  return values;
}

function assertDeniedWithoutTools(result, label) {
  assert(
    result.response.status === 401 || result.response.status === 403,
    `${label} should be denied, got ${describeResponse(result)}`,
  );
  const text = result.text || JSON.stringify(result.body ?? {});
  assert(
    !text.includes('propose_') && !text.includes('service_request:') && !text.includes('toolGroups'),
    `${label} leaked operator tools: ${text.slice(0, 1000)}`,
  );
}

function assertConfirmation(action, label) {
  assert(action, `missing ${label} action`);
  assert(
    action.kind === 'operator_confirmation' &&
      action.requiresConfirmation === true &&
      typeof action.confirmationId === 'string' &&
      action.confirmationId.length > 0,
    `${label} was not confirmation gated: ${JSON.stringify(action)}`,
  );
}

function findRequestAction(actions, requestAction) {
  return actions.find((action) => action?.request?.action === requestAction);
}

function summarizeAction(action) {
  return {
    kind: action.kind,
    requestAction: action.request?.action ?? null,
    requiresConfirmation: action.requiresConfirmation === true,
    confirmationId: action.confirmationId ?? null,
    params: action.request?.params ?? null,
  };
}

function compactTurn(data) {
  return {
    runId: data?.runId ?? null,
    toolCalls: (data?.toolCalls ?? []).map((call) => ({ tool: call.tool, status: call.status })),
    dashboardActions: (data?.dashboardActions ?? []).map((action) => ({
      kind: action.kind,
      requestAction: action.request?.action ?? null,
    })),
  };
}

function summarizeServiceResponse(payload) {
  const data = payload?.data ?? payload ?? {};
  return {
    browserId: firstString(recordValue(data, 'browserId'), recordValue(recordValue(data, 'browser'), 'id')),
    sessionId: firstString(recordValue(data, 'sessionId'), recordValue(recordValue(data, 'session'), 'id')),
    tabId: firstString(recordValue(data, 'tabId'), recordValue(data, 'targetId'), recordValue(recordValue(data, 'tab'), 'id')),
    jobId: firstString(recordValue(data, 'jobId'), recordValue(data, 'id')),
  };
}

function recordValue(source, key) {
  return source && typeof source === 'object' ? source[key] : undefined;
}

function firstString(...values) {
  for (const value of values) {
    if (typeof value === 'string' && value.trim()) return value.trim();
  }
  return null;
}

function describeResponse(result) {
  return `HTTP ${result.response.status} ${JSON.stringify(result.body ?? result.text).slice(0, 1000)}`;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function requiredValue(values, index, flag) {
  const value = values[index];
  if (!value) fail(`Missing value for ${flag}`);
  return value;
}

function fail(message) {
  console.error(message);
  process.exit(2);
}

function printHelp() {
  console.log(`Usage: node scripts/smoke-dashboard-operator-plan0022-live.js [options]

Options:
  --dashboard-url <url>       Dashboard URL to verify. Default: http://127.0.0.1:4848/
  --auth-env <path>           Dashboard auth env file. Default: ~/.agent-browser/dashboard-auth.env
  --run-live-operation        Attempt launch, DOM type/click, snapshot through Operate service actions.
  --run-ui-followup           Drive the dashboard UI and click View launched browser after launch.
  --require-live-operation    Fail if the launch/DOM operation cannot complete.
  --require-ui-followup       Fail if the dashboard UI follow-up cannot complete.
  --agent-browser-bin <path>  agent-browser binary used for UI follow-up smoke.
  --browser-profile <path>    Browser profile path used for UI follow-up smoke.
  --ui-session <name>         agent-browser session name used for UI follow-up smoke.
                              The UI driver forces --browser-host local_headless; the browser
                              launched by Operate still follows the service/default posture.
  --json                      Print structured JSON.
`);
}
