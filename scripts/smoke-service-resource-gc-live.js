#!/usr/bin/env node

import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  createMcpStdioClient,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import { assertTerminalRetirementState } from './service-resource-gc-live-contract.js';

const BIN = new URL('../cli/target/debug/agent-browser', import.meta.url).pathname;
const CHROME = existsSync('/opt/google/chrome/chrome')
  ? '/opt/google/chrome/chrome'
  : '/usr/bin/google-chrome';
const context = createSmokeContext({
  prefix: 'ab-managed-resource-gc-',
  sessionPrefix: 'managed-resource-gc',
});
context.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD = BIN;
context.env.AGENT_BROWSER_EXECUTABLE_PATH = CHROME;
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
context.env.AGENT_BROWSER_PROFILE_LEASE_MODE = 'fail_open_ephemeral';
context.env.AGENT_BROWSER_RESOURCE_INACTIVITY_MIN_SECONDS = '60';
delete context.env.AGENT_BROWSER_PROFILE;
delete context.env.AGENT_BROWSER_RUNTIME_PROFILE;

const statePath = join(context.agentHome, 'service', 'state.json');
const daemonPidPath = join(context.socketDir, 'runtime-host.pid');
const maintenanceSession = `${context.session}-maintenance`;
const protectedSession = `${context.session}-profile-holder`;
const unrelatedProfile = join(context.tempHome, 'unrelated-chrome');
let managedGroup;
let protectedGroup;
let unrelated;
let mcp;

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function pidRunning(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function groupRunning(processGroupId) {
  try {
    process.kill(-processGroupId, 0);
    return true;
  } catch {
    return false;
  }
}

function killGroup(processGroupId) {
  if (!processGroupId || !groupRunning(processGroupId)) return;
  try {
    process.kill(-processGroupId, 'SIGKILL');
  } catch {
    // Best effort cleanup for disposable processes only.
  }
}

function processGroupForPid(pid) {
  if (!Number.isInteger(pid) || pid <= 0 || !pidRunning(pid)) return undefined;
  const value = execFileSync('ps', ['-o', 'pgid=', '-p', String(pid)], { encoding: 'utf8' }).trim();
  const processGroupId = Number(value);
  return Number.isInteger(processGroupId) && processGroupId > 0 ? processGroupId : undefined;
}

function exactOwnedProcessGroups() {
  const rows = execFileSync('ps', ['-eo', 'pid=,pgid=,args='], { encoding: 'utf8' });
  const groups = new Set();
  for (const row of rows.trim().split('\n')) {
    const match = row.trim().match(/^(\d+)\s+(\d+)\s+(.*)$/);
    if (!match || !match[3].includes(context.tempHome)) continue;
    groups.add(Number(match[2]));
  }
  if (existsSync(daemonPidPath)) {
    const daemonPid = Number(readFileSync(daemonPidPath, 'utf8').trim());
    const daemonGroup = processGroupForPid(daemonPid);
    if (daemonGroup) groups.add(daemonGroup);
  }
  if (managedGroup) groups.add(managedGroup);
  if (protectedGroup) groups.add(protectedGroup);
  if (unrelated?.pid) {
    const unrelatedGroup = processGroupForPid(unrelated.pid);
    if (unrelatedGroup) groups.add(unrelatedGroup);
  }
  return [...groups];
}

function processGroupMembers(processGroupId) {
  const rows = execFileSync('ps', ['-eo', 'pid=,pgid='], { encoding: 'utf8' });
  return rows
    .trim()
    .split('\n')
    .map((row) => row.trim().split(/\s+/).map(Number))
    .filter(([, pgid]) => pgid === processGroupId)
    .map(([pid]) => pid);
}

async function runJson(args, label) {
  const result = await runCli(context, args, 120000);
  const parsed = parseJsonOutput(result.stdout, label);
  if (!parsed.success) throw new Error(`${label} failed: ${result.stdout}${result.stderr}`);
  return parsed.data;
}

async function readAuthoritativeState(label) {
  const status = await runJson(maintenanceArgs(['service', 'status', '--json']), label);
  const state = status.service_state ?? status.serviceState;
  if (!state || typeof state !== 'object') {
    throw new Error(`${label} returned no Service State: ${JSON.stringify(status)}`);
  }
  return state;
}

function candidates(data) {
  return data.actions?.retireAbandonedBrowserLane ?? [];
}

function parseToolPayload(result, label) {
  const text = result.content?.[0]?.text;
  if (typeof text !== 'string') throw new Error(`${label} returned no text payload`);
  return JSON.parse(text);
}

async function waitFor(predicate, label, timeoutMs = 10000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (predicate()) return;
    await sleep(100);
  }
  throw new Error(`Timed out waiting for ${label}`);
}

async function cleanup() {
  if (mcp) {
    mcp.close();
    mcp = undefined;
  }
  for (const processGroupId of exactOwnedProcessGroups()) killGroup(processGroupId);
  if (process.env.AGENT_BROWSER_SMOKE_PRESERVE === '1') {
    console.error(`Preserved smoke temp home: ${context.tempHome}`);
  } else {
    context.cleanupTempHome();
  }
}

function maintenanceArgs(args) {
  return ['--session', maintenanceSession, ...args];
}

async function main() {
  if (!existsSync(BIN)) throw new Error(`Build the source binary first: ${BIN}`);
  if (!existsSync(CHROME)) throw new Error(`Chrome executable not found: ${CHROME}`);

  const protectedOpened = await runJson(
    [
      '--json',
      '--session',
      protectedSession,
      'open',
      'data:text/html,<title>protected-profile-holder</title>',
    ],
    'protected profile-holder launch',
  );
  if (protectedOpened === undefined) throw new Error('Protected profile-holder launch returned no data');

  const streamEnabled = await runJson(
    ['--json', '--session', context.session, 'stream', 'enable'],
    'managed stream enable',
  );
  const servicePort = streamEnabled.port;
  if (!Number.isInteger(servicePort) || servicePort <= 0) {
    throw new Error(`Managed service port is invalid: ${JSON.stringify(streamEnabled)}`);
  }
  const serviceLaunch = await httpJson(servicePort, 'POST', '/api/service/request', {
    serviceName: 'AbandonedBrowserRetirementSmoke',
    agentName: 'acceptance-agent',
    taskName: 'retireDisposableBrowser',
    action: 'navigate',
    runtimeProfile: 'default',
    profileLeasePolicy: 'wait',
    profileLeaseWaitTimeoutMs: 1000,
    jobTimeoutMs: 120000,
    params: {
      url: 'data:text/html,<title>managed-gc</title>',
      waitUntil: 'load',
    },
  });
  if (!serviceLaunch.success) {
    throw new Error(`Managed Service request failed: ${JSON.stringify(serviceLaunch)}`);
  }
  await runJson(
    ['--json', '--session', context.session, 'service', 'reconcile'],
    'managed Service reconcile',
  );
  await waitFor(() => existsSync(statePath) && existsSync(daemonPidPath), 'managed state');

  const state = await readAuthoritativeState('managed authoritative Service State');
  const browserId = `session:${context.session}`;
  const protectedBrowserId = `session:${protectedSession}`;
  const browser = state.browsers?.[browserId];
  const lifecycle = state.runtimeOwnerRegistry?.lifecycleRecords?.[browserId];
  const protectedLifecycle = state.runtimeOwnerRegistry?.lifecycleRecords?.[protectedBrowserId];
  managedGroup = lifecycle?.processGroupId ?? processGroupForPid(browser?.pid);
  protectedGroup =
    protectedLifecycle?.processGroupId ?? processGroupForPid(state.browsers?.[protectedBrowserId]?.pid);
  if (!browser?.pid || !lifecycle?.processGroupId || !lifecycle.packageLaunchIdentityDigest) {
    throw new Error(`Managed lifecycle identity is incomplete: ${JSON.stringify({ browser, lifecycle })}`);
  }
  const disposableProfile = state.profiles?.[browser.profileId];
  if (disposableProfile?.profileClass !== 'managed_one_time' || disposableProfile.persistent) {
    throw new Error(`Lease conflict did not create a disposable profile: ${JSON.stringify(disposableProfile)}`);
  }
  managedGroup = lifecycle.processGroupId;
  const managedMembersBefore = processGroupMembers(managedGroup);
  if (managedMembersBefore.length < 2) {
    throw new Error(`Expected a managed Chrome helper tree; got ${managedMembersBefore.join(',')}`);
  }

  const serviceSessionId =
    browser.activeSessionIds?.[0] ??
    Object.values(state.sessions ?? {}).find((session) => session.browserIds?.includes(browserId))?.id;
  if (!serviceSessionId) {
    throw new Error(`Managed browser has no service session: ${JSON.stringify(browser)}`);
  }
  mcp = createMcpStdioClient({
    context,
    args: ['--session', context.session, 'mcp', 'serve'],
    onFatal: (message) => {
      throw new Error(message);
    },
  });
  await mcp.send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'agent-browser-abandoned-retirement-smoke', version: '0' },
  });
  mcp.notify('notifications/initialized');
  const inactiveAt = new Date(Date.now() - 5 * 60 * 1000).toISOString();
  const sessionUpsert = parseToolPayload(
    await mcp.send('tools/call', {
      name: 'service_session_upsert',
      arguments: {
        id: serviceSessionId,
        session: {
          lease: 'expired',
          cleanup: 'close_browser',
          browserIds: [browserId],
          tabIds: state.sessions?.[serviceSessionId]?.tabIds ?? [],
          lastLeaseObservedAt: inactiveAt,
          expiresAt: inactiveAt,
        },
        serviceName: 'AbandonedBrowserRetirementSmoke',
        agentName: 'acceptance-agent',
        taskName: 'retireDisposableBrowser',
      },
    }),
    'service_session_upsert',
  );
  if (!sessionUpsert.success) {
    throw new Error(`Could not expire the disposable session: ${JSON.stringify(sessionUpsert)}`);
  }
  mcp.close();
  mcp = undefined;

  mkdirSync(unrelatedProfile, { recursive: true });
  unrelated = spawn(
    CHROME,
    [
      '--headless=new',
      '--no-sandbox',
      '--remote-debugging-port=0',
      `--user-data-dir=${unrelatedProfile}`,
      'about:blank',
    ],
    { detached: true, stdio: 'ignore' },
  );
  unrelated.unref();
  await waitFor(() => unrelated.pid && pidRunning(unrelated.pid), 'unrelated Chrome');

  const daemonPid = Number(readFileSync(daemonPidPath, 'utf8').trim());
  if (!Number.isInteger(daemonPid) || daemonPid <= 0) {
    throw new Error(`Invalid disposable daemon PID: ${daemonPid}`);
  }
  process.kill(daemonPid, 'SIGKILL');
  await waitFor(() => !pidRunning(daemonPid), 'disposable daemon exit');
  if (!groupRunning(managedGroup)) throw new Error('Managed Chrome did not survive daemon loss');

  const dryRun = await runJson(
    maintenanceArgs(['service', 'gc', '--dry-run', '--json']),
    'managed GC dry-run',
  );
  const reviewed = candidates(dryRun);
  if (reviewed.length !== 1 || reviewed[0].pid !== browser.pid) {
    const resources = await runJson(
      maintenanceArgs(['service', 'resources', '--json']),
      'managed resources diagnostics',
    );
    const diagnosticState = await readAuthoritativeState('managed GC diagnostic Service State');
    const relevantResources = resources.resources?.filter(
      (resource) =>
        resource.pid === browser.pid ||
        resource.pid === unrelated.pid ||
        resource.correlation?.browserId === browserId,
    );
    throw new Error(
      `Expected only managed PID ${browser.pid}; got ${JSON.stringify(reviewed)} resources=${JSON.stringify(relevantResources)} lanes=${JSON.stringify(resources.lanes)} sessions=${JSON.stringify(diagnosticState.sessions)} tabs=${JSON.stringify(diagnosticState.tabs)} browser=${JSON.stringify(diagnosticState.browsers?.[browserId])} profile=${JSON.stringify(diagnosticState.profiles?.[browser.profileId])}`,
    );
  }
  if (!pidRunning(unrelated.pid)) throw new Error('Unrelated Chrome exited before GC apply');
  if (!groupRunning(protectedGroup)) throw new Error('Protected profile-holder exited before GC apply');
  const apply = await runJson(
    maintenanceArgs([
      'service',
      'gc',
      '--apply',
      '--review-token',
      dryRun.reviewToken,
      '--json',
    ]),
    'managed GC apply',
  );
  if (apply.counts?.retiredAbandonedBrowserLanes !== 1 || apply.counts?.failed !== 0) {
    throw new Error(`Unexpected managed GC result: ${JSON.stringify(apply)}`);
  }
  await waitFor(() => !groupRunning(managedGroup), 'managed Chrome process-group exit');
  const profileRoot = state.browserProcessIdentities?.[browserId]?.userDataDir;
  if (!profileRoot || existsSync(join(profileRoot, 'SingletonLock'))) {
    throw new Error('Managed Chrome profile lock survived GC');
  }
  const terminalState = await readAuthoritativeState('terminal authoritative Service State');
  assertTerminalRetirementState(terminalState, browserId);
  if (!pidRunning(unrelated.pid)) throw new Error('GC terminated unrelated Chrome');
  if (!groupRunning(protectedGroup)) throw new Error('GC terminated the protected profile-holder');

  killGroup(unrelated.pid);
  unrelated = undefined;
  managedGroup = undefined;
  await cleanup();
  console.log(
    `service-resource-gc-live: ok managed_pid=${browser.pid} helpers=${managedMembersBefore.length} unrelated_protected=true terminal_receipt=true`,
  );
}

main().catch(async (error) => {
  console.error(`service-resource-gc-live: ${error.message}`);
  await cleanup();
  process.exit(1);
});
