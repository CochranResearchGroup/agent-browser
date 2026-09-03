#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { mkdir, open, readFile, readlink } from 'node:fs/promises';
import { dirname, isAbsolute, join, resolve } from 'node:path';

import { sha256 } from './lib/p158-campaign-controller.js';
import {
  aggregateP158DashboardCampaignReceipts,
  buildP158DashboardIngressSelectorRequest,
  buildP158DashboardCampaignPlan,
  buildP158DashboardFixtureFromProjection,
  executeP158DashboardCampaignAction,
  prepareP158DashboardCampaign,
  resolveP158DashboardActionResume,
  sha256Bytes,
  validateP158ExternalPlaywrightRunner,
} from './lib/p158-w8-dashboard-campaign.js';
import {
  applyP158DashboardScenarioToFixture,
  sealP158DashboardScenarioReceipt,
} from './lib/p158-w8-dashboard-scenarios.js';
import {
  auditP158DashboardLiveProjection,
  buildP158DashboardExternalProof,
  captureP158DashboardLiveProjection,
} from './lib/p158-w8-dashboard-live.js';
import {
  buildP158DashboardGithubRunnerAttestation,
  sealP158DashboardExternalResult,
  validateP158DashboardExternalActionUrl,
  validateP158DashboardExternalManifest,
} from './lib/p158-w8-dashboard-external.js';
import {
  pauseP158DashboardHostAction,
  resumeP158DashboardHostAction,
} from './lib/p158-w8-dashboard-host-handshake.js';

const argv = process.argv.slice(2).filter((argument) => argument !== '--');
const command = argv.shift();

function option(name, required = true) {
  const index = argv.indexOf(name);
  if (index === -1) {
    if (required) throw new Error(`${name} is required`);
    return null;
  }
  const value = argv[index + 1];
  if (!value || value.startsWith('--')) throw new Error(`${name} requires a value`);
  argv.splice(index, 2);
  return value;
}

function flag(name) {
  const index = argv.indexOf(name);
  if (index === -1) return false;
  argv.splice(index, 1);
  return true;
}

async function jsonFile(path, label) {
  if (!isAbsolute(path)) throw new Error(`${label} path must be absolute`);
  return JSON.parse(await readFile(path, 'utf8'));
}

async function writeNewJson(path, value) {
  if (!isAbsolute(path)) throw new Error('Output path must be absolute');
  await mkdir(dirname(path), { recursive: true, mode: 0o700 });
  const handle = await open(path, 'wx', 0o600);
  try {
    await handle.writeFile(`${JSON.stringify(value, null, 2)}\n`);
  } finally {
    await handle.close();
  }
}

function parseJson(text, label) {
  try {
    return JSON.parse(text.trim());
  } catch (error) {
    throw new Error(`${label} did not return JSON: ${error.message}`);
  }
}

function runProcess(executable, args, { env, input = '' } = {}) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(executable, args, {
      env,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.on('error', reject);
    child.on('exit', (code, signal) => {
      if (code === 0) resolvePromise({ stdout, stderr });
      else reject(new Error(`${executable} failed with code=${code} signal=${signal}: ${stderr}`));
    });
    child.stdin.end(input);
  });
}

async function assertExecutableDigest(candidate) {
  const bytes = await readFile(candidate.executablePath);
  if (sha256Bytes(bytes) !== candidate.executableSha256) {
    throw new Error('Frozen candidate executable digest changed');
  }
}

function isolatedEnvironment(root) {
  return {
    PATH: process.env.PATH ?? '/usr/bin:/bin',
    LANG: process.env.LANG ?? 'C.UTF-8',
    NO_COLOR: '1',
    ...root.environment,
  };
}

async function processIdentity(pid, candidateSha256) {
  const stat = await readFile(`/proc/${pid}/stat`, 'utf8');
  const fields = stat.slice(stat.lastIndexOf(') ') + 2).trim().split(/\s+/);
  const executablePath = await readlink(`/proc/${pid}/exe`);
  const executableSha256 = sha256Bytes(await readFile(`/proc/${pid}/exe`));
  if (executableSha256 !== candidateSha256) throw new Error(`PID ${pid} does not run the frozen candidate`);
  return { pid, startToken: fields[19], executablePath, executableSha256 };
}

async function waitForProcessIdentity(pid, candidateSha256) {
  let lastError = null;
  for (let observation = 0; observation < 50; observation += 1) {
    try {
      return await processIdentity(pid, candidateSha256);
    } catch (error) {
      lastError = error;
      await new Promise((resolvePromise) => setTimeout(resolvePromise, 100));
    }
  }
  throw lastError ?? new Error(`PID ${pid} identity did not become observable`);
}

async function assertSameProcessIdentity(expected) {
  const current = await processIdentity(expected.pid, expected.executableSha256);
  if (current.startToken !== expected.startToken || current.executablePath !== expected.executablePath) {
    throw new Error(`PID ${expected.pid} was reused or changed executable identity`);
  }
  return current;
}

async function validateInstalledState(request) {
  await assertExecutableDigest(request.candidate);
  await mkdir(dirname(request.validationInputPath), { recursive: true, mode: 0o700 });
  const handle = await open(request.validationInputPath, 'wx', 0o600);
  try {
    await handle.writeFile(request.stateBytes);
  } finally {
    await handle.close();
  }
  const result = await runProcess(request.candidate.executablePath, [
    'service', 'state', 'validate', '--path', request.validationInputPath, '--json',
  ], { env: {
    PATH: process.env.PATH ?? '/usr/bin:/bin',
    LANG: process.env.LANG ?? 'C.UTF-8',
    NO_COLOR: '1',
    ...request.environment,
  } });
  return parseJson(result.stdout, 'installed Service State validator');
}

async function selectorReceipt(selector, request) {
  if (!isAbsolute(selector?.executablePath ?? '') || !isAbsolute(selector?.sourcePath ?? '') ||
      !/^[a-f0-9]{64}$/.test(selector?.executableSha256 ?? '') ||
      !/^[a-f0-9]{64}$/.test(selector?.sourceSha256 ?? '')) {
    throw new Error('Execution input requires an absolute reviewed development ingress selector source and executable identity');
  }
  const [executableBytes, sourceBytes] = await Promise.all([
    readFile(selector.executablePath), readFile(selector.sourcePath),
  ]);
  if (sha256Bytes(executableBytes) !== selector.executableSha256 ||
      sha256Bytes(sourceBytes) !== selector.sourceSha256) {
    throw new Error('Development ingress selector source or executable digest changed');
  }
  const args = request.operation === 'select' ? ['--apply'] : [];
  const result = await runProcess(selector.executablePath, args, {
    env: { PATH: process.env.PATH ?? '/usr/bin:/bin', LANG: process.env.LANG ?? 'C.UTF-8' },
    input: `${JSON.stringify({
      schemaVersion: 'agent-browser.p158-development-ingress-selection-request.v1',
      ...request,
    })}\n`,
  });
  return parseJson(result.stdout, 'development ingress selector');
}

function dashboardSelectionUrl(publicUrl, key, value) {
  const selected = new URL(publicUrl);
  for (const candidate of ['workspace', 'browser', 'session', 'tab', 'profile', 'job']) {
    selected.searchParams.delete(candidate);
  }
  selected.searchParams.set(key, value);
  return selected.href;
}

function dashboardSectionUrl(publicUrl, section) {
  const selected = new URL(publicUrl);
  const segments = selected.pathname.split('/').filter(Boolean);
  if (['service', 'browsers', 'activity'].includes(segments.at(-1))) segments.pop();
  selected.pathname = section === 'overview'
    ? `/${segments.join('/')}`
    : `/${[...segments, section].join('/')}`;
  return selected.href;
}

async function ensureDetailPane(page) {
  const toggle = page.getByRole('button', { name: 'Show detail pane', exact: true });
  if (await toggle.isVisible()) await toggle.click();
}

async function waitForWorkspace(page, expectedWorkspaceId) {
  await page.waitForFunction((expected) => {
    const selected = document.querySelector('.workspace-nav-row-main[aria-current="true"]');
    const inspector = document.querySelector('section[aria-label="Selected workspace details"]');
    return selected?.getAttribute('data-workspace-id') === expected &&
      inspector?.getAttribute('data-selected-workspace-id') === expected;
  }, expectedWorkspaceId);
  return page.evaluate(() => {
    const selected = document.querySelector('.workspace-nav-row-main[aria-current="true"]');
    const inspector = document.querySelector('section[aria-label="Selected workspace details"]');
    return {
      selectedWorkspaceId: selected?.getAttribute('data-workspace-id') ?? null,
      inspectorWorkspaceId: inspector?.getAttribute('data-selected-workspace-id') ?? null,
      state: inspector?.getAttribute('data-selected-workspace-state') ?? null,
    };
  });
}

function browserIdFromWorkspace(workspaceId) {
  return typeof workspaceId === 'string' && workspaceId.startsWith('browser:')
    ? workspaceId.slice('browser:'.length)
    : workspaceId;
}

async function exerciseD03({ pageHandle, publicUrl, scenarioPlan }) {
  const page = pageHandle.page;
  const truth = scenarioPlan.scenarioTruth;
  await page.goto(dashboardSelectionUrl(
    dashboardSectionUrl(publicUrl, 'service'), 'profile', truth.expectedSelectedResourceId,
  ), {
    waitUntil: 'networkidle',
  });
  await ensureDetailPane(page);
  const rowSelector = (resourceId) => `button[aria-label="Inspect profile allocation ${resourceId}"]`;
  await page.waitForSelector(rowSelector(truth.expectedSelectedResourceId));
  const duplicateRows = await page.evaluate(({ resourceIds, duplicateLabel }) => resourceIds.map((resourceId) => {
    const button = document.querySelector(`button[aria-label="Inspect profile allocation ${CSS.escape(resourceId)}"]`);
    const labels = Array.from(button?.querySelectorAll('span') ?? [])
      .map((entry) => (entry.textContent ?? '').trim())
      .filter(Boolean);
    return { resourceId, label: labels.find((entry) => entry === duplicateLabel) ?? null };
  }), { resourceIds: truth.duplicateResourceIds, duplicateLabel: truth.duplicateLabel });
  await page.locator(rowSelector(truth.expectedSelectedResourceId)).click();
  await page.waitForFunction((resourceId) => document
    .querySelector(`button[aria-label="Inspect profile allocation ${CSS.escape(resourceId)}"]`)
    ?.getAttribute('aria-current') === 'true', truth.expectedSelectedResourceId);
  await page.waitForFunction((resourceId) => document
    .querySelector('.service-inspector[data-inspector-kind="profile"]')
    ?.getAttribute('data-inspector-resource-id') === resourceId, truth.expectedSelectedResourceId);
  const selectedResourceId = await page.evaluate(() => {
    const selected = document.querySelector('button[aria-label^="Inspect profile allocation "][aria-current="true"]');
    return selected?.getAttribute('aria-label')?.slice('Inspect profile allocation '.length) ?? null;
  });
  const browsers = await page.evaluate(async () => {
    const response = await fetch('/api/service/browsers?limit=500', {
      credentials: 'same-origin', cache: 'no-store',
    });
    const body = await response.json();
    if (!response.ok || body.success !== true) throw new Error('D03 browser collection failed');
    return body.data?.browsers ?? body.data ?? [];
  });
  const byId = new Map(browsers.map((entry) => [entry.id, entry]));
  const crossProfileBindings = truth.crossProfileBindings.map(({ browserId }) => ({
    browserId,
    profileId: byId.get(browserId)?.profileId ?? null,
  }));
  const inspectorResourceId = await page.evaluate(() => document
    .querySelector('.service-inspector[data-inspector-kind="profile"]')
    ?.getAttribute('data-inspector-resource-id') ?? null);
  const actionButton = page.locator('button[data-action-id="show-browser"]');
  await actionButton.waitFor();
  const actionTargetResourceId = await actionButton.getAttribute('data-resource-id');
  await actionButton.click();
  await waitForWorkspace(page, `browser:${truth.expectedActionTargetResourceId}`);
  return sealP158DashboardScenarioReceipt({
    schemaVersion: 'agent-browser.p158-dashboard-scenario-receipt.v1',
    actionId: scenarioPlan.actionId,
    caseId: scenarioPlan.caseId,
    scenarioPlanSha256: scenarioPlan.scenarioPlanSha256,
    duplicateRows,
    crossProfileBindings,
    selectedResourceId,
    inspectorResourceId,
    actionTargetResourceId,
    wrongResourceSelected: selectedResourceId !== truth.expectedSelectedResourceId,
    wrongResourceActioned: actionTargetResourceId !== truth.expectedActionTargetResourceId,
    repairAttempted: false,
    retryAttempted: false,
    garbageCollectionAttempted: false,
  });
}

async function exerciseD04({ pageHandle, publicUrl, publicPath, selectionReceiptSha256,
  externalProof, scenarioPlan }) {
  const clients = [];
  for (const expected of scenarioPlan.scenarioTruth.clients) {
    const overviewUrl = dashboardSectionUrl(publicUrl, 'overview');
    const client = await pageHandle.openClient(expected.clientId, overviewUrl);
    await ensureDetailPane(client);
    const selectedUrl = dashboardSelectionUrl(overviewUrl, 'browser', expected.expectedSelectedResourceId);
    const alternateUrl = dashboardSelectionUrl(overviewUrl, 'browser', expected.alternateResourceId);
    await client.goto(selectedUrl, { waitUntil: 'networkidle' });
    let selection = await waitForWorkspace(client, `browser:${expected.expectedSelectedResourceId}`);
    const observedSelectedResourceId = browserIdFromWorkspace(selection.selectedWorkspaceId);
    const observedInspectorResourceId = browserIdFromWorkspace(selection.inspectorWorkspaceId);
    await client.reload({ waitUntil: 'networkidle' });
    selection = await waitForWorkspace(client, `browser:${expected.expectedSelectedResourceId}`);
    const selectionAfterRefresh = browserIdFromWorkspace(selection.selectedWorkspaceId);
    await client.goto(alternateUrl, { waitUntil: 'networkidle' });
    await waitForWorkspace(client, `browser:${expected.alternateResourceId}`);
    await client.goBack({ waitUntil: 'networkidle' });
    await waitForWorkspace(client, `browser:${expected.expectedSelectedResourceId}`);
    await client.goForward({ waitUntil: 'networkidle' });
    await waitForWorkspace(client, `browser:${expected.alternateResourceId}`);
    await client.goBack({ waitUntil: 'networkidle' });
    selection = await waitForWorkspace(client, `browser:${expected.expectedSelectedResourceId}`);
    const selectionAfterBackForward = browserIdFromWorkspace(selection.selectedWorkspaceId);
    await client.goto(selectedUrl, { waitUntil: 'networkidle' });
    selection = await waitForWorkspace(client, `browser:${expected.expectedSelectedResourceId}`);
    const selectionAfterDeepLink = browserIdFromWorkspace(selection.selectedWorkspaceId);
    clients.push({
      clientId: expected.clientId,
      offHost: externalProof?.offHost === true,
      outsideServiceNetworkNamespace: externalProof?.outsideServiceNetworkNamespace === true,
      clientIngressReceiptSha256: sha256({
        actionId: scenarioPlan.actionId,
        clientId: expected.clientId,
        publicPath,
        selectionReceiptSha256,
        offHost: externalProof?.offHost === true,
        outsideServiceNetworkNamespace: externalProof?.outsideServiceNetworkNamespace === true,
      }),
      completedOperations: structuredClone(expected.operations),
      expectedSelectedResourceId: expected.expectedSelectedResourceId,
      observedSelectedResourceId,
      observedInspectorResourceId,
      selectionAfterRefresh,
      selectionAfterBackForward,
      selectionAfterDeepLink,
      finalBarrierSelectedResourceId: null,
      finalBarrierInspectorResourceId: null,
    });
  }
  for (const [index, expected] of scenarioPlan.scenarioTruth.clients.entries()) {
    const client = pageHandle.clientPage(expected.clientId);
    const finalSelection = await waitForWorkspace(client, `browser:${expected.expectedSelectedResourceId}`);
    clients[index].finalBarrierSelectedResourceId = browserIdFromWorkspace(finalSelection.selectedWorkspaceId);
    clients[index].finalBarrierInspectorResourceId = browserIdFromWorkspace(finalSelection.inspectorWorkspaceId);
  }
  await pageHandle.page.goto(dashboardSectionUrl(publicUrl, 'service'), { waitUntil: 'networkidle' });
  return sealP158DashboardScenarioReceipt({
    schemaVersion: 'agent-browser.p158-dashboard-scenario-receipt.v1',
    actionId: scenarioPlan.actionId,
    caseId: scenarioPlan.caseId,
    scenarioPlanSha256: scenarioPlan.scenarioPlanSha256,
    publicPath,
    selectionReceiptSha256,
    clients,
    repairAttempted: false,
    retryAttempted: false,
    garbageCollectionAttempted: false,
  });
}

async function exerciseD05({ pageHandle, publicUrl, scenarioPlan }) {
  const page = pageHandle.page;
  const truth = scenarioPlan.scenarioTruth;
  await page.goto(dashboardSectionUrl(publicUrl, 'overview'), { waitUntil: 'networkidle' });
  await ensureDetailPane(page);
  const initialStaleSelectionObserved = await page.evaluate(({ browserId, staleRequestedId }) => {
    const calls = [];
    const originalReplaceState = window.history.replaceState.bind(window.history);
    window.history.replaceState = (state, unused, url) => {
      const before = new URL(window.location.href).searchParams.get('tab');
      originalReplaceState(state, unused, url);
      const after = new URL(window.location.href).searchParams.get('tab');
      calls.push({ before, after });
    };
    window.__p158DashboardRecoveryReplaceCalls = calls;
    const next = new URL(window.location.href);
    next.searchParams.set('browser', browserId);
    next.searchParams.set('tab', staleRequestedId);
    next.searchParams.set('view', 'workspace:view');
    window.history.pushState({ p158StaleSelection: true }, '', `${next.pathname}${next.search}${next.hash}`);
    const detail = {
      workspaceId: null, browserId, sessionId: null, tabId: staleRequestedId,
      profileId: null, jobId: null,
    };
    window.dispatchEvent(new CustomEvent('agent-browser-dashboard-workspace-selection-change', { detail }));
    return new URL(window.location.href).searchParams.get('tab') === staleRequestedId;
  }, { browserId: truth.selectedBrowserId, staleRequestedId: truth.staleRequestedId });
  await page.waitForFunction(({ expectedSelectionId, staleRequestedId }) => {
    const params = new URL(window.location.href).searchParams;
    const explanation = Array.from(document.querySelectorAll('.workspace-remote-viewport-notice'))
      .map((entry) => entry.textContent ?? '').find((text) => text.includes(staleRequestedId));
    return params.get('tab') === expectedSelectionId && Boolean(explanation);
  }, { expectedSelectionId: truth.expectedResolvedSelectionId, staleRequestedId: truth.staleRequestedId });
  const recovered = await waitForWorkspace(page, truth.expectedWorkspaceId);
  const recovery = await page.evaluate(({ staleRequestedId, expectedSelectionId }) => {
    const matchingCalls = (window.__p158DashboardRecoveryReplaceCalls ?? []).filter((entry) =>
      entry.before === staleRequestedId && entry.after === expectedSelectionId);
    const explanation = Array.from(document.querySelectorAll('.workspace-remote-viewport-notice'))
      .map((entry) => (entry.textContent ?? '').trim())
      .find((text) => text.includes(staleRequestedId) && text.includes(expectedSelectionId)) ?? null;
    return {
      recoveryEventCount: matchingCalls.length,
      resolvedSelectionId: new URL(window.location.href).searchParams.get('tab'),
      recoveryExplanation: explanation,
    };
  }, { staleRequestedId: truth.staleRequestedId, expectedSelectionId: truth.expectedResolvedSelectionId });
  const scenarioReceipt = sealP158DashboardScenarioReceipt({
    schemaVersion: 'agent-browser.p158-dashboard-scenario-receipt.v1',
    actionId: scenarioPlan.actionId,
    caseId: scenarioPlan.caseId,
    scenarioPlanSha256: scenarioPlan.scenarioPlanSha256,
    queryKey: truth.queryKey,
    staleRequestedId: truth.staleRequestedId,
    initialStaleSelectionObserved,
    recoveryEventCount: recovery.recoveryEventCount,
    recoveryMethod: 'dashboard_history_replace',
    recoveryExplanation: recovery.recoveryExplanation,
    resolvedSelectionId: recovery.resolvedSelectionId,
    resolvedResourceId: browserIdFromWorkspace(recovered.selectedWorkspaceId),
    resolvedWorkspaceId: recovered.inspectorWorkspaceId,
    repairAttempted: false,
    retryAttempted: false,
    garbageCollectionAttempted: false,
  });
  await page.goto(dashboardSectionUrl(publicUrl, 'service'), { waitUntil: 'networkidle' });
  return scenarioReceipt;
}

async function openDirectExternalPage(publicUrl) {
  const { chromium } = await import('playwright');
  const browser = await chromium.launch({ headless: true });
  const clients = new Map();
  const openClient = async (clientId, clientUrl) => {
    if (clients.has(clientId)) throw new Error(`Dashboard client ${clientId} was opened twice`);
    if (clients.size === 1 && clients.has('primary')) {
      const primary = clients.get('primary');
      clients.delete('primary');
      clients.set(clientId, primary);
      await primary.page.goto(clientUrl, { waitUntil: 'networkidle' });
      return primary.page;
    }
    const context = await browser.newContext();
    const page = await context.newPage();
    clients.set(clientId, { context, page });
    await page.goto(clientUrl, { waitUntil: 'networkidle' });
    const username = process.env.AGENT_BROWSER_P158_DASHBOARD_USERNAME;
    const password = process.env.AGENT_BROWSER_P158_DASHBOARD_PASSWORD;
    if (username || password) {
      if (!username || !password) throw new Error('Both external dashboard credential variables are required');
      const authenticated = await page.evaluate(async ({ login, secret }) => {
        const response = await fetch('/api/dashboard-auth/login', {
          method: 'POST', credentials: 'same-origin', headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ username: login, password: secret }),
        });
        return response.ok;
      }, { login: username, secret: password });
      if (!authenticated) throw new Error('External dashboard authentication failed');
      await page.reload({ waitUntil: 'networkidle' });
    }
    return page;
  };
  try {
    const page = await openClient('primary', publicUrl);
    return {
      page,
      openClient,
      clientPage: (clientId) => {
        const client = clients.get(clientId);
        if (!client) throw new Error(`Dashboard client ${clientId} is not open`);
        return client.page;
      },
      close: async () => {
        for (const { context } of clients.values()) await context.close();
        await browser.close();
      },
    };
  } catch (error) {
    for (const { context } of clients.values()) await context.close();
    await browser.close();
    throw error;
  }
}

function createLiveEffects(executionInput) {
  return {
    startExact: async (root) => {
      await assertExecutableDigest(root.candidate);
      await Promise.all(Object.entries(root.environment)
        .filter(([key]) => key === 'HOME' || key.startsWith('XDG_') || key === 'AGENT_BROWSER_SOCKET_DIR')
        .map(([, path]) => mkdir(path, { recursive: true, mode: 0o700 })));
      const runtimeHost = spawn(root.candidate.executablePath, [], {
        env: {
          ...isolatedEnvironment(root),
          AGENT_BROWSER_RUNTIME_HOST: '1',
          AGENT_BROWSER_RUNTIME_HOST_PROCESS: '1',
        },
        detached: true,
        stdio: 'ignore',
      });
      runtimeHost.unref();
      const runtimeHostIdentity = await waitForProcessIdentity(
        runtimeHost.pid,
        root.candidate.executableSha256,
      );
      let result;
      try {
        result = await runProcess(root.candidate.executablePath, [
          '--json', 'dashboard', 'start', '--port', String(root.ports.dashboardIngress),
        ], { env: isolatedEnvironment(root) });
      } catch (error) {
        await assertSameProcessIdentity(runtimeHostIdentity);
        process.kill(runtimeHost.pid, 'SIGTERM');
        throw error;
      }
      const parsed = parseJson(result.stdout, 'dashboard start');
      const pid = parsed?.data?.pid;
      const backendPid = parsed?.data?.backendPid;
      if (parsed?.success !== true || !Number.isInteger(pid) || !Number.isInteger(backendPid) ||
          parsed?.data?.backendPort !== root.ports.dashboardBackend) {
        throw new Error('Dashboard start omitted its exact ingress or backend process');
      }
      const processIdentities = {
        ingress: await waitForProcessIdentity(pid, root.candidate.executableSha256),
        backend: await waitForProcessIdentity(backendPid, root.candidate.executableSha256),
        runtimeHost: runtimeHostIdentity,
      };
      return {
        state: 'ready',
        pid,
        backendPid,
        runtimeHostPid: runtimeHost.pid,
        processIdentities,
        candidateSha256: root.candidate.executableSha256,
        statePath: root.target.statePath,
      };
    },
    selectExternalIngress: (request) => selectorReceipt(executionInput.campaignPlan.ingressSelector, request),
    observeExactRuntime: async ({ checkpoint, root }) => {
      try {
        const processIdentities = Object.fromEntries(await Promise.all(
          Object.entries(checkpoint.processIdentities).map(async ([role, identity]) =>
            [role, await assertSameProcessIdentity(identity)]),
        ));
        return {
          unchanged: true,
          processIdentities,
          runtimeRootSha256: sha256(root.target.disposableRoot),
          statePathSha256: sha256(root.target.statePath),
          ports: structuredClone(root.ports),
          candidateSha256: root.candidate.executableSha256,
        };
      } catch (error) {
        return { unchanged: false, code: error.code ?? 'host_runtime_identity_lost' };
      }
    },
    observeExactIngress: async ({ checkpoint, root }) => {
      try {
        return await selectorReceipt(executionInput.campaignPlan.ingressSelector,
          buildP158DashboardIngressSelectorRequest({
            root, processIdentities: checkpoint.processIdentities, operation: 'observe',
          }));
      } catch (error) {
        return { unchanged: false, code: error.code ?? 'host_ingress_identity_lost' };
      }
    },
    openExternalPage: async ({ publicUrl }) => {
      const endpoint = process.env.AGENT_BROWSER_P158_EXTERNAL_PLAYWRIGHT_WS;
      if (!endpoint) throw new Error('AGENT_BROWSER_P158_EXTERNAL_PLAYWRIGHT_WS is required for off-host capture');
      const externalRunner = validateP158ExternalPlaywrightRunner({
        endpoint,
        attestation: executionInput.externalRunnerAttestation,
      });
      const { chromium } = await import('playwright');
      const browser = await chromium.connect(endpoint);
      const clients = new Map();
      const openClient = async (clientId, clientUrl) => {
        if (clients.has(clientId)) throw new Error(`Dashboard client ${clientId} was opened twice`);
        if (clients.size === 1 && clients.has('primary')) {
          const primary = clients.get('primary');
          clients.delete('primary');
          clients.set(clientId, primary);
          await primary.page.goto(clientUrl, { waitUntil: 'networkidle' });
          return primary.page;
        }
        const context = await browser.newContext();
        const page = await context.newPage();
        clients.set(clientId, { context, page });
        await page.goto(clientUrl, { waitUntil: 'networkidle' });
        const username = process.env.AGENT_BROWSER_P158_DASHBOARD_USERNAME;
        const password = process.env.AGENT_BROWSER_P158_DASHBOARD_PASSWORD;
        if (username || password) {
          if (!username || !password) throw new Error('Both external dashboard credential variables are required');
          const authenticated = await page.evaluate(async ({ login, secret }) => {
            const response = await fetch('/api/dashboard-auth/login', {
              method: 'POST',
              credentials: 'same-origin',
              headers: { 'content-type': 'application/json' },
              body: JSON.stringify({ username: login, password: secret }),
            });
            return response.ok;
          }, { login: username, secret: password });
          if (!authenticated) throw new Error('External dashboard authentication failed');
          await page.reload({ waitUntil: 'networkidle' });
        }
        return page;
      };
      let page;
      try {
        page = await openClient('primary', publicUrl);
      } catch (error) {
        for (const { context } of clients.values()) await context.close();
        await browser.close();
        throw error;
      }
      return {
        page,
        externalRunner,
        openClient,
        clientPage: (clientId) => {
          const client = clients.get(clientId);
          if (!client) throw new Error(`Dashboard client ${clientId} is not open`);
          return client.page;
        },
        close: async () => {
          for (const { context } of clients.values()) await context.close();
          await browser.close();
        },
      };
    },
    exerciseScenario: async (request) => {
      if (request.scenarioPlan.caseId === 'D03') return exerciseD03(request);
      if (request.scenarioPlan.caseId === 'D04') return exerciseD04(request);
      return exerciseD05(request);
    },
    produceChurn: async () => ({
      blocked: true,
      detail: 'Installed Service has no declared lock-respecting development fixture mutation API; raw state replacement is prohibited',
      retryAttempted: false,
    }),
    stopExact: async ({ expectedPid, environment, processIdentities }) => {
      const root = { environment };
      const pidPath = join(environment.AGENT_BROWSER_SOCKET_DIR, 'dashboard.pid');
      const backendPidPath = join(environment.AGENT_BROWSER_SOCKET_DIR, 'dashboard-backend.pid');
      const selectedPid = Number((await readFile(pidPath, 'utf8')).trim());
      const selectedBackendPid = Number((await readFile(backendPidPath, 'utf8')).trim());
      if (selectedPid !== expectedPid || selectedBackendPid !== processIdentities.backend.pid) {
        throw new Error('Dashboard PID selectors do not match the started instance');
      }
      await assertSameProcessIdentity(processIdentities.ingress);
      await assertSameProcessIdentity(processIdentities.backend);
      await assertSameProcessIdentity(processIdentities.runtimeHost);
      const result = await runProcess(executionInput.campaignPlan.candidate.executablePath,
        ['--json', 'dashboard', 'stop'],
        { env: isolatedEnvironment(root) });
      const parsed = parseJson(result.stdout, 'dashboard stop');
      if (parsed?.success !== true || parsed?.data?.stopped !== true ||
          parsed?.data?.ingressStopped !== true || parsed?.data?.backendStopped !== true) {
        throw new Error('Dashboard stop did not stop both selected dashboard processes');
      }
      process.kill(processIdentities.runtimeHost.pid, 'SIGTERM');
      return {
        state: 'stopped',
        pid: expectedPid,
        backendPid: processIdentities.backend.pid,
        runtimeHostPid: processIdentities.runtimeHost.pid,
      };
    },
  };
}

async function main() {
  const inputPath = option('--input');
  const outputPath = option('--output');
  const apply = flag('--apply');
  if (argv.length > 0) throw new Error(`Unexpected arguments: ${argv.join(' ')}`);
  const input = await jsonFile(resolve(inputPath), 'Input');
  let output;
  if (command === 'external-capture') {
    if (!apply) throw new Error('external-capture requires --apply');
    const manifest = validateP158DashboardExternalManifest(input);
    let publicUrl = null;
    let runnerAttestation = null;
    let pageHandle = null;
    let terminalError = null;
    try {
      runnerAttestation = buildP158DashboardGithubRunnerAttestation(process.env);
      publicUrl = validateP158DashboardExternalActionUrl({
        manifest,
        publicUrl: process.env.AGENT_BROWSER_P158_DASHBOARD_ACTION_URL,
      });
      pageHandle = await openDirectExternalPage(publicUrl);
      const request = {
        pageHandle,
        publicUrl,
        publicPath: manifest.publicPath,
        selectionReceiptSha256: manifest.selectionReceiptSha256,
        externalProof: buildP158DashboardExternalProof({ publicUrl, runnerAttestation }),
        scenarioPlan: manifest.scenarioPlan,
      };
      const scenarioReceipt = manifest.caseId === 'D03'
        ? await exerciseD03(request)
        : manifest.caseId === 'D04' ? await exerciseD04(request) : await exerciseD05(request);
      const projection = await captureP158DashboardLiveProjection({
        page: pageHandle.page,
        materializationReceipt: manifest.materializationReceipt,
        externalProof: request.externalProof,
        screenshotPath: join(dirname(resolve(outputPath)), 'dashboard.png'),
      });
      let dashboardFixture = buildP158DashboardFixtureFromProjection({
        projection,
        actionId: manifest.actionId,
        expectedState: manifest.expectedState,
        materializationReceipt: manifest.materializationReceipt,
      });
      dashboardFixture = applyP158DashboardScenarioToFixture({
        fixture: dashboardFixture,
        plan: manifest.scenarioPlan,
        receipt: scenarioReceipt,
      });
      const oracleBinding = auditP158DashboardLiveProjection({ projection, dashboardFixture });
      output = sealP158DashboardExternalResult({
        manifest, scenarioReceipt, runnerAttestation, projection, dashboardFixture, oracleBinding,
      });
    } catch (error) {
      terminalError = error;
      output = sealP158DashboardExternalResult({
        manifest,
        failure: {
          code: error.code ?? 'external_capture_failed',
          message: 'External dashboard capture failed; consult the restricted workflow log',
        },
      });
    } finally {
      if (pageHandle) {
        try {
          await pageHandle.close();
        } catch (error) {
          terminalError ??= error;
          if (output?.success === true) {
            output = sealP158DashboardExternalResult({
              manifest,
              failure: {
                code: error.code ?? 'external_capture_teardown_failed',
                message: 'External dashboard capture teardown failed; consult the restricted workflow log',
              },
            });
          }
        }
      }
    }
    await writeNewJson(resolve(outputPath), output);
    if (terminalError) throw terminalError;
    process.stdout.write(`${JSON.stringify({ success: true, outputPath: resolve(outputPath) })}\n`);
    return;
  } else if (command === 'host-pause') {
    if (!apply) throw new Error('host-pause requires --apply');
    if (!isAbsolute(input.externalManifestOutputPath ?? '')) {
      throw new Error('host-pause requires an absolute externalManifestOutputPath');
    }
    const effects = createLiveEffects(input);
    effects.persistDispatchReady = async ({ checkpoint, externalManifest }) => {
      await writeNewJson(input.externalManifestOutputPath, externalManifest);
      await writeNewJson(resolve(outputPath), checkpoint);
    };
    const paused = await pauseP158DashboardHostAction({
      ...input,
      effects,
    });
    if (paused.state === 'dispatch_ready') {
      process.stdout.write(`${JSON.stringify({ success: true, outputPath: resolve(outputPath) })}\n`);
      return;
    }
    output = paused;
  } else if (command === 'host-resume') {
    if (!apply) throw new Error('host-resume requires --apply');
    output = await resumeP158DashboardHostAction({
      ...input,
      effects: createLiveEffects(input),
    });
  } else if (command === 'plan') {
    if (apply) throw new Error('plan does not accept --apply');
    output = buildP158DashboardCampaignPlan(input);
  } else if (command === 'prepare') {
    output = await prepareP158DashboardCampaign({ ...input, apply, validateState: validateInstalledState });
  } else if (command === 'execute') {
    if (!apply) throw new Error('execute requires --apply');
    const effects = createLiveEffects(input);
    const receipts = [];
    for (const root of input.campaignPlan.roots) {
      const receiptPath = join(dirname(resolve(outputPath)), 'action-receipts', `${sha256Bytes(root.actionId).slice(0, 24)}.json`);
      const claimPath = join(dirname(resolve(outputPath)), 'action-claims', `${sha256Bytes(root.actionId).slice(0, 24)}.json`);
      let existingReceipt = null;
      let existingClaim = null;
      try {
        existingReceipt = JSON.parse(await readFile(receiptPath, 'utf8'));
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      try {
        existingClaim = JSON.parse(await readFile(claimPath, 'utf8'));
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      const resume = resolveP158DashboardActionResume({
        campaignPlan: input.campaignPlan,
        actionId: root.actionId,
        claim: existingClaim,
        receipt: existingReceipt,
      });
      if (resume.disposition === 'reuse_terminal') {
        receipts.push(resume.receipt);
        continue;
      }
      await writeNewJson(claimPath, {
        schemaVersion: 'agent-browser.p158-dashboard-action-claim.v1',
        actionId: root.actionId,
        campaignPlanSha256: input.campaignPlan.campaignPlanSha256,
        effectState: 'claimed_uncertain_until_terminal_receipt',
      });
      const receipt = await executeP158DashboardCampaignAction({
        campaignPlan: input.campaignPlan,
        preparation: input.preparation,
        freezeState: input.freezeState,
        actionId: root.actionId,
        effects,
      });
      receipts.push(receipt);
      await writeNewJson(receiptPath, receipt);
    }
    output = {
      receipts,
      aggregate: aggregateP158DashboardCampaignReceipts({ campaignPlan: input.campaignPlan, receipts }),
    };
  } else {
    throw new Error('Command must be plan, prepare, host-pause, host-resume, execute, or external-capture');
  }
  await writeNewJson(resolve(outputPath), output);
  process.stdout.write(`${JSON.stringify({ success: true, outputPath: resolve(outputPath) })}\n`);
}

main().catch((error) => {
  process.stderr.write(`${JSON.stringify({ success: false, code: error.code ?? 'runner_failed', error: error.message })}\n`);
  process.exitCode = 1;
});
