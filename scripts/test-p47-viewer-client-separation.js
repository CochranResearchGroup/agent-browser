#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  commandVectorUsesServiceOwnership,
  createViewerClientLaunchDescriptor,
  dashboardCredentialsFromEnv,
  dashboardWorkspaceUrl,
  parseEnvText,
  readDevToolsActivePort,
  resolveViewerClientExecutable,
  resolveViewerClientDebuggingPort,
  verifiedChromiumFromInstallDoctor,
} from './lib/p47-viewer-client.js';

const moduleSource = readFileSync('scripts/lib/p47-viewer-client.js', 'utf8');
const runnerSource = readFileSync('scripts/run-p46-stress-scenario.js', 'utf8');
const reconnectSource = moduleSource.slice(
  moduleSource.indexOf('function reconnectDashboardViewerClient'),
  moduleSource.indexOf('export async function waitForDashboardState'),
);

assert.doesNotMatch(
  moduleSource,
  /AGENT_BROWSER_COMMAND|runAgent|runAgentSession|remote-view\s+open|spawnSync|service\s+status/i,
  'viewer-client module must not execute service-owned target-browser commands',
);

assert.match(
  runnerSource,
  /launchDashboardViewerClient/,
  'P46 S2 must use the viewer-client module for dashboard operator browsers',
);

assert.match(
  moduleSource,
  /waitForDevToolsActivePort/,
  'viewer-client launch must read Chromium dynamic DevTools port evidence',
);

assert.match(
  moduleSource,
  /CDP command \$\{method\} timed out after 30000ms/,
  'viewer-client CDP commands must have bounded timeouts',
);

assert.match(
  moduleSource,
  /function navigateDashboardViewerClient[\s\S]*history\.pushState[\s\S]*PopStateEvent[\s\S]*window\.location\.assign/,
  'viewer-client dashboard navigation must use same-origin history swap before falling back to location.assign',
);

assert.match(
  moduleSource,
  /function reconnectDashboardViewerClient[\s\S]*\/json[\s\S]*viewerClient\.cdp = nextCdp/,
  'viewer-client dashboard navigation must reconnect CDP after swapped page navigation',
);
assert.match(
  moduleSource,
  /function waitForDashboardViewerClientPageUrl[\s\S]*expectedUrl[\s\S]*urlMatched[\s\S]*writeJson\(artifactName, last\)/,
  'viewer-client dashboard swap must wait for DevTools page URL evidence before reconnect',
);
assert.match(
  moduleSource,
  /recoveredStaleTabState[\s\S]*Recovered stale selected tab identity[\s\S]*allowRecoveredStaleTab/,
  'viewer-client dashboard state wait must expose a narrow stale-tab recovery proof option',
);
assert.match(
  moduleSource,
  /chosenPage[\s\S]*previousPage[\s\S]*samePageId[\s\S]*writeJson\(artifactName, discovery\)/,
  'viewer-client reconnect must write target discovery evidence before reconnecting CDP',
);
assert.doesNotMatch(
  reconnectSource,
  /nextCdp\.send\('Page\.enable'\)|nextCdp\.send\('Runtime\.enable'\)/,
  'viewer-client reconnect must not send Page.enable before dashboard-state readback',
);

assert.doesNotMatch(
  moduleSource,
  /Math\.random\(\)\s*\*\s*20000/,
  'viewer-client launch must not choose a random fixed DevTools port by default',
);

assert.doesNotMatch(
  runnerSource,
  /launchExternalDashboardOperator|evalInSession|dashboardStateScript\(/,
  'P46 runner must not retain the old agent-browser-session dashboard operator path',
);

const installDoctorJson = {
  data: {
    launchConfig: {
      browserBuildManifests: {
        stealthcdp_chromium: {
          ready: true,
          executablePathExists: true,
          executablePath: process.execPath,
        },
      },
    },
  },
};

assert.equal(
  verifiedChromiumFromInstallDoctor(installDoctorJson),
  process.execPath,
  'verified Chromium resolver should prefer ready install-doctor browser build path',
);

const resolved = resolveViewerClientExecutable({
  commandExists: (candidate) => candidate,
  env: {},
  installDoctorJson,
});
assert.equal(resolved.executable, process.execPath);
assert.equal(resolved.verifiedChromium, process.execPath);

const artifactDir = mkdtempSync(join(tmpdir(), 'agent-browser-p47-viewer-client-'));
try {
  assert.equal(resolveViewerClientDebuggingPort({}), 0);
  assert.equal(resolveViewerClientDebuggingPort({ P47_VIEWER_CLIENT_REMOTE_DEBUGGING_PORT: '43211' }), 43211);
  assert.throws(
    () => resolveViewerClientDebuggingPort({ P47_VIEWER_CLIENT_REMOTE_DEBUGGING_PORT: 'not-a-port' }),
    /Invalid viewer-client DevTools port override/,
  );

  const descriptor = createViewerClientLaunchDescriptor({
    artifactDir,
    dashboardUrl: dashboardWorkspaceUrl({
      browserId: 'session:default',
      sessionName: 'default',
      tabId: 'target:1',
    }),
    executable: resolved.executable,
    label: 'operator-a',
    port: 43210,
    profileDir: join(artifactDir, 'profile'),
    verifiedChromium: resolved.verifiedChromium,
  });

  assert.equal(descriptor.role, 'viewer-client');
  assert.equal(descriptor.forbiddenServiceOwnership, true);
  assert.equal(descriptor.executable, process.execPath);
  assert.equal(descriptor.readinessUrl, 'http://127.0.0.1:43210/json/version');
  assert.ok(descriptor.launchArgs.includes('--headless=new'));
  assert.ok(descriptor.launchArgs.includes('--disable-gpu'));
  assert.ok(descriptor.launchArgs.includes('--no-sandbox'));
  assert.ok(descriptor.launchArgs.some((arg) => arg.startsWith('--user-data-dir=')));
  assert.ok(descriptor.launchArgs.some((arg) => arg === '--remote-debugging-port=43210'));
  assert.equal(commandVectorUsesServiceOwnership(descriptor.executable, descriptor.launchArgs), false);

  const dynamicDescriptor = createViewerClientLaunchDescriptor({
    artifactDir,
    dashboardUrl: descriptor.dashboardUrl,
    executable: resolved.executable,
    label: 'operator-b',
    port: 0,
    profileDir: join(artifactDir, 'dynamic-profile'),
    verifiedChromium: resolved.verifiedChromium,
  });
  assert.equal(dynamicDescriptor.remoteDebuggingPortMode, 'chromium_dynamic');
  assert.equal(dynamicDescriptor.readinessUrl, null);
  assert.ok(dynamicDescriptor.launchArgs.some((arg) => arg === '--remote-debugging-port=0'));

  const activePortProfileDir = join(artifactDir, 'active-port-profile');
  mkdirSync(activePortProfileDir);
  writeFileSync(join(activePortProfileDir, 'DevToolsActivePort'), '45678\n/devtools/browser/example\n');
  assert.deepEqual(readDevToolsActivePort(activePortProfileDir), {
    browserPath: '/devtools/browser/example',
    path: join(activePortProfileDir, 'DevToolsActivePort'),
    port: 45678,
  });
} finally {
  rmSync(artifactDir, { recursive: true, force: true });
}

assert.equal(commandVectorUsesServiceOwnership('agent-browser', ['--json', 'open', 'https://example.com']), true);
assert.equal(commandVectorUsesServiceOwnership('agent-browser', ['--json', 'remote-view', 'open']), true);
assert.equal(commandVectorUsesServiceOwnership('google-chrome', ['--session', 'operator-a']), true);
assert.equal(commandVectorUsesServiceOwnership('google-chrome', ['service', 'request']), true);
assert.equal(commandVectorUsesServiceOwnership('google-chrome', ['--remote-debugging-port=43210']), false);

const env = parseEnvText(`
AGENT_BROWSER_DASHBOARD_CODEX_USERNAME=codex
AGENT_BROWSER_DASHBOARD_CODEX_PASSWORD='secret value'
`);
const credentials = dashboardCredentialsFromEnv(env, '/tmp/dashboard-auth.env');
assert.equal(credentials.ok, true);
assert.equal(credentials.username, 'codex');
assert.equal(credentials.password, 'secret value');
assert.equal(credentials.path, '/tmp/dashboard-auth.env');

console.log('P47 viewer-client separation no-live checks passed');
