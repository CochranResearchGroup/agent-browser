#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { loadAgentBrowserEnvFromRealHome } from './smoke-remote-headed-utils.js';

loadAgentBrowserEnvFromRealHome();

const args = process.argv.slice(2);
const options = {
  agentBrowserBin: process.env.AGENT_BROWSER_BIN || 'agent-browser',
  dashboardUrl: process.env.AGENT_BROWSER_DASHBOARD_URL || 'https://agent-browser.ecochran.dyndns.org/',
  json: false,
  keepProfile: false,
  routeLabel: '',
  session: `remote-view-target-attribution-${process.pid}`,
  targetUrl: 'https://example.com',
};

for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === '--') {
    continue;
  } else if (arg === '--agent-browser-bin') {
    options.agentBrowserBin = requiredValue(args, ++index, arg);
  } else if (arg === '--dashboard-url') {
    options.dashboardUrl = requiredValue(args, ++index, arg);
  } else if (arg === '--json') {
    options.json = true;
  } else if (arg === '--keep-profile') {
    options.keepProfile = true;
  } else if (arg === '--route') {
    options.routeLabel = requiredValue(args, ++index, arg).toUpperCase();
  } else if (arg === '--session') {
    options.session = requiredValue(args, ++index, arg);
  } else if (arg === '--target-url') {
    options.targetUrl = requiredValue(args, ++index, arg);
  } else if (arg === '--help' || arg === '-h') {
    printHelp();
    process.exit(0);
  } else {
    fail(`Unknown argument: ${arg}`);
  }
}

const tempDir = mkdtempSync(join(tmpdir(), 'agent-browser-plan0025-m6-'));
const browserProfile = join(tempDir, 'browser-profile');
const artifactDir = mkdtempSync(join(tmpdir(), 'agent-browser-plan0025-m6-artifacts-'));
mkdirSync(artifactDir, { recursive: true });
const report = {
  dashboardUrl: options.dashboardUrl,
  selectedWorkspaceId: `daemon-session:${options.session}`,
  browserId: `session:${options.session}`,
  session: options.session,
  targetUrl: options.targetUrl,
  route: null,
  initialDisplay: null,
  launch: null,
  hostedDashboard: null,
  visibleDisplay: null,
  closeResult: null,
  cleanup: null,
  artifactDir,
};

const timeout = setTimeout(() => {
  failWithReport('Timed out waiting for remote view target attribution smoke to complete');
}, 540000);

try {
  await run();
  clearTimeout(timeout);
  output({ success: true, ...report });
} catch (error) {
  clearTimeout(timeout);
  cleanupTarget();
  recordFinalDisplayProbe();
  cleanupProfile();
  output({
    success: false,
    error: error instanceof Error ? error.message : String(error),
    ...report,
  });
  process.exit(1);
}

async function run() {
  const initial = inspectRouteDisplays();
  const route = selectRoute(initial);
  report.route = routeSummary(route);
  report.initialDisplay = route.displayContent;
  assert(
    route.displayContent?.state === 'terminal_only',
    `Selected route ${route.label} is not terminal-only before launch: ${JSON.stringify(route.displayContent)}`,
  );

  const launch = runAgent([
    '--json',
    '--session',
    options.session,
    '--profile',
    browserProfile,
    '--browser-host',
    'remote_headed',
    '--view-stream-provider',
    'rdp_gateway',
    '--control-input-provider',
    'manual_attached_desktop',
    '--display-isolation',
    'shared_display',
    'open',
    options.targetUrl,
  ], {
    env: {
      AGENT_BROWSER_REMOTE_HEADED_DISPLAY: route.displayName,
    },
    timeoutMs: 120000,
  });
  const launchJson = parseJson(launch.stdout, 'remote headed target launch');
  assert(launchJson.success === true, `Remote-headed target launch failed: ${launch.stdout}${launch.stderr}`);
  const status = runAgent(['--json', '--session', options.session, 'service', 'status'], { timeoutMs: 60000 });
  const statusJson = parseJson(status.stdout, 'service status after target launch');
  const browser = statusJson.data?.service_state?.browsers?.[`session:${options.session}`] ?? null;
  const stream = browser?.viewStreams?.find((candidate) => candidate.provider === 'rdp_gateway') ?? browser?.viewStreams?.[0] ?? null;
  const screenshotPath = join(artifactDir, 'target-visible.png');
  const screenshot = runAgent(['--json', '--session', options.session, 'screenshot', screenshotPath], { timeoutMs: 60000 });
  const screenshotJson = parseJson(screenshot.stdout, 'target screenshot');
  assert(screenshotJson.success === true, `Target screenshot failed: ${screenshot.stdout}${screenshot.stderr}`);
  report.launch = {
    title: launchJson.data?.title ?? null,
    url: launchJson.data?.url ?? null,
    displayName: route.displayName,
    streamProvider: stream?.provider ?? 'rdp_gateway',
    routeId: stream?.routeId ?? `display:${route.displayName}`,
    streamUrl: stream?.frameUrl ?? stream?.url ?? stream?.externalUrl ?? null,
    screenshotPath,
  };

  const visible = inspectRouteDisplays();
  const visibleRoute = routeByLabel(visible, route.label);
  report.visibleDisplay = visibleRoute?.displayContent ?? null;
  assert(
    visibleRoute?.displayContent?.state === 'browser_window_visible',
    `Route ${route.label} did not report a visible browser window after launch: ${JSON.stringify(visibleRoute?.displayContent)}`,
  );

  const hosted = runNode([
    'scripts/smoke-local-dashboard-runtime.js',
    '--dashboard-url',
    options.dashboardUrl,
    '--agent-browser-bin',
    options.agentBrowserBin,
    '--browser-host',
    'local_headless',
    '--browser-profile',
    join(tempDir, 'dashboard-viewer-profile'),
    '--workspace-session',
    options.session,
    '--skip-chat',
    '--json',
  ], { timeoutMs: 360000 });
  const hostedJson = parseJson(hosted.stdout, 'hosted dashboard workspace smoke');
  assert(hostedJson.success === true, `Hosted dashboard workspace smoke failed: ${hosted.stdout}${hosted.stderr}`);
  const workspaceState = hostedJson.browser?.workspaceState;
  const expectedWorkspaceIds = [
    `daemon-session:${options.session}`,
    `browser:session:${options.session}`,
  ];
  assert(
    workspaceState?.hasWorkspaceDetail === true &&
      expectedWorkspaceIds.includes(workspaceState?.selectedWorkspaceId),
    `Hosted dashboard did not select the expected workspace: ${JSON.stringify(workspaceState)}`,
  );
  report.hostedDashboard = {
    http: hostedJson.http,
    selectedWorkspaceId: workspaceState.selectedWorkspaceId,
    selectedWorkspaceState: workspaceState.selectedWorkspaceState,
    frameSrc: hostedJson.browser?.frameSrc ?? null,
    readinessStatus: hostedJson.browser?.readinessStatus ?? null,
  };

  cleanupTarget();
  const final = inspectRouteDisplays();
  const finalRoute = routeByLabel(final, route.label);
  report.cleanup = finalRoute?.displayContent ?? null;
  assert(
    finalRoute?.displayContent?.state === 'terminal_only',
    `Route ${route.label} did not return to terminal-only after cleanup: ${JSON.stringify(finalRoute?.displayContent)}`,
  );

  writeFileSync(join(artifactDir, 'remote-view-target-attribution.json'), `${JSON.stringify(report, null, 2)}\n`);
  cleanupProfile();
}

function inspectRouteDisplays() {
  const result = runNode(['scripts/inspect-rdp-route-displays.js', '--windows'], { timeoutMs: 15000 });
  const parsed = parseJson(result.stdout, 'route display inspection');
  assert(parsed.success === true, `Route display inspection failed: ${result.stdout}${result.stderr}`);
  return parsed;
}

function selectRoute(reportValue) {
  const routes = Object.entries(reportValue.routes ?? {})
    .map(([label, route]) => ({ ...route, label }))
    .filter((route) => route.displayName);
  if (options.routeLabel) {
    const route = routes.find((item) => item.label === options.routeLabel);
    assert(route, `Route ${options.routeLabel} was not present: ${JSON.stringify(reportValue.routes)}`);
    return route;
  }
  const terminalOnly = routes.find((route) => route.displayContent?.state === 'terminal_only');
  assert(terminalOnly, `No terminal-only route display is available for the regression smoke: ${JSON.stringify(reportValue.routes)}`);
  return terminalOnly;
}

function routeByLabel(reportValue, label) {
  const route = reportValue.routes?.[label];
  return route ? { ...route, label } : null;
}

function routeSummary(route) {
  return {
    label: route.label,
    displayName: route.displayName,
    user: route.user,
    candidateCount: route.candidates?.length ?? 0,
  };
}

function cleanupTarget() {
  if (report.closeResult) return;
  try {
    const close = runAgent(['--json', '--session', options.session, 'close'], { timeoutMs: 30000 });
    const closeJson = parseJson(close.stdout, 'target cleanup close');
    report.closeResult = {
      closeSuccess: closeJson.success === true,
      closeData: closeJson.data ?? null,
    };
  } catch (error) {
    report.closeResult = {
      closeSuccess: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

function cleanupProfile() {
  if (options.keepProfile) return;
  rmSync(tempDir, { recursive: true, force: true });
}

function runAgent(commandArgs, { env = {}, timeoutMs = 60000 } = {}) {
  return runCommand(options.agentBrowserBin, commandArgs, {
    env,
    label: `agent-browser ${commandArgs.join(' ')}`,
    timeoutMs,
  });
}

function runNode(commandArgs, { timeoutMs = 60000 } = {}) {
  return runCommand('node', commandArgs, {
    label: `node ${commandArgs.join(' ')}`,
    timeoutMs,
  });
}

function runCommand(command, commandArgs, { env = {}, label, timeoutMs }) {
  const result = spawnSync(command, commandArgs, {
    cwd: new URL('..', import.meta.url),
    env: { ...process.env, ...env },
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    stdio: 'pipe',
    timeout: timeoutMs,
  });
  if (result.error) {
    throw new Error(`${label} failed: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`${label} failed with status ${result.status}: ${result.stdout}${result.stderr}`);
  }
  return result;
}

function parseJson(text, label) {
  try {
    return JSON.parse(String(text).trim());
  } catch (error) {
    throw new Error(`Failed to parse ${label} JSON: ${error instanceof Error ? error.message : String(error)}\n${text}`);
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function output(value) {
  if (options.json) {
    console.log(JSON.stringify(value, null, 2));
  } else if (value.success) {
    console.log(`Remote view target attribution live smoke passed: ${value.route?.label} ${value.route?.displayName}`);
  } else {
    console.error(value.error);
  }
}

function failWithReport(message) {
  cleanupTarget();
  recordFinalDisplayProbe();
  cleanupProfile();
  output({ success: false, error: message, ...report });
  process.exit(1);
}

function recordFinalDisplayProbe() {
  if (report.cleanup || !report.route?.label) return;
  try {
    const final = inspectRouteDisplays();
    const finalRoute = routeByLabel(final, report.route.label);
    report.cleanup = finalRoute?.displayContent ?? null;
  } catch (error) {
    report.cleanup = {
      state: 'display_probe_unavailable',
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

function fail(message) {
  console.error(message);
  process.exit(2);
}

function requiredValue(values, index, flag) {
  const value = values[index];
  if (!value) fail(`Missing value for ${flag}`);
  return value;
}

function printHelp() {
  console.log(`Usage: node scripts/smoke-remote-view-target-attribution-live.js [options]

Options:
  --dashboard-url <url>       Hosted dashboard URL. Default: AGENT_BROWSER_DASHBOARD_URL or https://agent-browser.ecochran.dyndns.org/
  --agent-browser-bin <path>  agent-browser binary used for launch and hosted browser smoke.
  --route <A|B>               Require a specific route label. Default: first terminal-only route.
  --session <name>            Disposable target session name.
  --target-url <url>          Target URL to launch into the route. Default: https://example.com
  --keep-profile              Keep temporary smoke profiles and artifacts.
  --json                      Print structured JSON.
`);
}
