#!/usr/bin/env node

import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { createSmokeContext, parseJsonOutput, runCli } from './smoke-utils.js';

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

const statePath = join(context.agentHome, 'service', 'state.json');
const ownerRegistryPath = join(context.agentHome, 'service', 'runtime-owner-registry.json');
const daemonPidPath = join(context.socketDir, `${context.session}.pid`);
const maintenanceSession = `${context.session}-maintenance`;
const maintenanceDaemonPidPath = join(context.socketDir, `${maintenanceSession}.pid`);
const unrelatedProfile = join(context.tempHome, 'unrelated-chrome');
let managedGroup;
let unrelated;

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

function processGroupMembers(processGroupId) {
  const rows = execFileSync('ps', ['-eo', 'pid=,pgid='], { encoding: 'utf8' });
  return rows
    .trim()
    .split('\n')
    .map((row) => row.trim().split(/\s+/).map(Number))
    .filter(([, pgid]) => pgid === processGroupId)
    .map(([pid]) => pid);
}

function readState() {
  return JSON.parse(readFileSync(statePath, 'utf8'));
}

function writeState(state) {
  writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`);
  writeFileSync(
    ownerRegistryPath,
    `${JSON.stringify(
      {
        schemaVersion: 'agent-browser.runtime-owner-registry.v1',
        registry: state.runtimeOwnerRegistry,
      },
      null,
      2,
    )}\n`,
  );
}

async function runJson(args, label) {
  const result = await runCli(context, args, 120000);
  const parsed = parseJsonOutput(result.stdout, label);
  if (!parsed.success) throw new Error(`${label} failed: ${result.stdout}${result.stderr}`);
  return parsed.data;
}

function candidates(data) {
  return data.actions?.terminateProcess ?? [];
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
  killGroup(managedGroup);
  if (unrelated?.pid) killGroup(unrelated.pid);
  if (existsSync(maintenanceDaemonPidPath)) {
    const maintenancePid = Number(readFileSync(maintenanceDaemonPidPath, 'utf8').trim());
    if (Number.isInteger(maintenancePid) && maintenancePid > 0 && pidRunning(maintenancePid)) {
      process.kill(maintenancePid, 'SIGKILL');
    }
  }
  context.cleanupTempHome();
}

function maintenanceArgs(args) {
  return ['--session', maintenanceSession, ...args];
}

async function main() {
  if (!existsSync(BIN)) throw new Error(`Build the source binary first: ${BIN}`);
  if (!existsSync(CHROME)) throw new Error(`Chrome executable not found: ${CHROME}`);

  const opened = await runJson(
    ['--json', '--session', context.session, 'open', 'data:text/html,<title>managed-gc</title>'],
    'managed browser launch',
  );
  if (opened === undefined) throw new Error('Managed browser launch returned no data');
  await waitFor(() => existsSync(statePath) && existsSync(daemonPidPath), 'managed state');

  let state = readState();
  const browserId = `session:${context.session}`;
  const browser = state.browsers?.[browserId];
  const lifecycle = state.runtimeOwnerRegistry?.lifecycleRecords?.[browserId];
  if (!browser?.pid || !lifecycle?.processGroupId || !lifecycle.packageLaunchIdentityDigest) {
    throw new Error(`Managed lifecycle identity is incomplete: ${JSON.stringify({ browser, lifecycle })}`);
  }
  managedGroup = lifecycle.processGroupId;
  const managedMembersBefore = processGroupMembers(managedGroup);
  if (managedMembersBefore.length < 2) {
    throw new Error(`Expected a managed Chrome helper tree; got ${managedMembersBefore.join(',')}`);
  }

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

  state = readState();
  const originalGeneration = state.runtimeOwnerRegistry.lifecycleRecords[browserId].ownerGeneration;
  state.browsers[browserId].health = 'faulted';
  state.browsers[browserId].activeSessionIds = [];
  state.runtimeOwnerRegistry.lifecycleRecords[browserId].lifecycleState = 'closing';
  state.runtimeOwnerRegistry.lifecycleRecords[browserId].cleanupObligationState = 'owned';
  writeState(state);
  await sleep(6000);

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
    const diagnosticState = readState();
    const relevantResources = resources.resources?.filter(
      (resource) =>
        resource.pid === browser.pid ||
        resource.pid === unrelated.pid ||
        resource.correlation?.browserId === browserId,
    );
    throw new Error(
      `Expected only managed PID ${browser.pid}; got ${JSON.stringify(reviewed)} resources=${JSON.stringify(relevantResources)} projectedLanes=${JSON.stringify(resources.runtimeLanes)} identity=${JSON.stringify(diagnosticState.browserProcessIdentities?.[browserId])} registry=${JSON.stringify(diagnosticState.runtimeOwnerRegistry)}`,
    );
  }
  if (!pidRunning(unrelated.pid)) throw new Error('Unrelated Chrome exited before GC apply');

  state = readState();
  state.runtimeOwnerRegistry.lifecycleRecords[browserId].ownerGeneration = originalGeneration + 1;
  writeState(state);
  let driftRejected = false;
  try {
    await runJson(
      maintenanceArgs([
        'service',
        'gc',
        '--apply',
        '--review-token',
        dryRun.reviewToken,
        '--json',
      ]),
      'generation drift rejection',
    );
  } catch (error) {
    driftRejected = error.message.includes('review_token_candidate_mismatch');
  }
  if (!driftRejected || !groupRunning(managedGroup)) {
    throw new Error(`Generation drift did not block GC: rejected=${driftRejected}`);
  }

  state = readState();
  state.runtimeOwnerRegistry.lifecycleRecords[browserId].ownerGeneration = originalGeneration;
  writeState(state);
  const fresh = await runJson(
    maintenanceArgs(['service', 'gc', '--dry-run', '--json']),
    'fresh managed GC dry-run',
  );
  const apply = await runJson(
    maintenanceArgs([
      'service',
      'gc',
      '--apply',
      '--review-token',
      fresh.reviewToken,
      '--json',
    ]),
    'managed GC apply',
  );
  if (apply.counts?.terminated !== 1 || apply.counts?.skipped !== 0 || apply.counts?.failed !== 0) {
    throw new Error(`Unexpected managed GC result: ${JSON.stringify(apply)}`);
  }
  await waitFor(() => !groupRunning(managedGroup), 'managed Chrome process-group exit');
  const profileRoot = state.browserProcessIdentities?.[browserId]?.userDataDir;
  if (!profileRoot || existsSync(join(profileRoot, 'SingletonLock'))) {
    throw new Error('Managed Chrome profile lock survived GC');
  }
  if (!pidRunning(unrelated.pid)) throw new Error('GC terminated unrelated Chrome');

  killGroup(unrelated.pid);
  unrelated = undefined;
  managedGroup = undefined;
  await cleanup();
  console.log(
    `service-resource-gc-live: ok managed_pid=${browser.pid} helpers=${managedMembersBefore.length} unrelated_protected=true drift_rejected=true`,
  );
}

main().catch(async (error) => {
  console.error(`service-resource-gc-live: ${error.message}`);
  await cleanup();
  process.exit(1);
});
