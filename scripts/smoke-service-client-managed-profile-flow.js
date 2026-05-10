#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  rootDir,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-managed-profile-flow-',
  sessionPrefix: 'managed-profile-flow',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
context.env.AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS = '0';

const { agentHome, session, tempHome } = context;
const serviceName = 'CanvaCLI';
const agentName = 'canva-cli-agent';
const taskName = 'openCanvaWorkspace';
const targetServiceId = 'canva';
const profileId = `managed-profile-flow-${process.pid}`;
const monitorId = `managed-profile-flow-freshness-${process.pid}`;
const userDataDir = join(tempHome, 'managed-profile-flow-user-data');

const timeout = setTimeout(() => {
  fail('Timed out waiting for managed profile flow smoke to complete');
}, 90000);

function seedServiceState() {
  const serviceDir = join(agentHome, 'service');
  mkdirSync(serviceDir, { recursive: true });
  writeFileSync(
    join(serviceDir, 'state.json'),
    `${JSON.stringify(
      {
        profiles: {
          [profileId]: {
            id: profileId,
            name: 'Managed profile flow smoke profile',
            userDataDir,
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [targetServiceId],
            sharedServiceIds: [serviceName],
            targetReadiness: [
              {
                targetServiceId,
                loginId: targetServiceId,
                state: 'fresh',
                manualSeedingRequired: false,
                evidence: 'auth_probe_cookie_present',
                recommendedAction: 'use_profile',
                lastVerifiedAt: '2026-05-10T12:00:00Z',
                freshnessExpiresAt: '2999-05-01T00:00:01Z',
              },
            ],
            persistent: true,
          },
        },
        monitors: {
          [monitorId]: {
            id: monitorId,
            name: 'Managed profile flow freshness',
            target: { profile_readiness: targetServiceId },
            intervalMs: 60000,
            state: 'active',
            lastCheckedAt: null,
            lastSucceededAt: null,
            lastFailedAt: null,
            lastResult: null,
            consecutiveFailures: 0,
          },
        },
      },
      null,
      2,
    )}\n`,
  );
}

async function cleanup() {
  clearTimeout(timeout);
  try {
    await closeSession(context);
  } finally {
    if (process.env.AGENT_BROWSER_SMOKE_KEEP_HOME === '1') {
      console.error(`Keeping smoke home: ${tempHome}`);
    } else {
      context.cleanupTempHome();
    }
  }
}

async function fail(message) {
  await cleanup();
  console.error(message);
  process.exit(1);
}

async function ensureStreamPort() {
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
  assert(Number.isInteger(port) && port > 0, `stream status did not return a port: ${JSON.stringify(stream)}`);
  return port;
}

function runManagedProfileFlow(args) {
  return new Promise((resolve, reject) => {
    const proc = spawn(process.execPath, ['examples/service-client/managed-profile-flow.mjs', ...args], {
      cwd: rootDir,
      env: context.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const procTimeout = setTimeout(() => {
      proc.kill('SIGTERM');
      reject(new Error(`managed profile flow timed out: ${args.join(' ')}`));
    }, 60000);
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
        reject(new Error(`managed profile flow failed: code=${code} signal=${signal}\n${out}${err}`));
      }
    });
  });
}

function readServiceState() {
  return JSON.parse(readFileSync(join(agentHome, 'service', 'state.json'), 'utf8'));
}

try {
  seedServiceState();

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status');
  assert(status.success === true, `service status failed: ${statusResult.stdout}${statusResult.stderr}`);

  const port = await ensureStreamPort();
  const pageUrl = smokeDataUrl('Managed Profile Flow Smoke', 'Managed Profile Flow Smoke');
  const flowResult = await runManagedProfileFlow([
    '--base-url',
    `http://127.0.0.1:${port}`,
    '--service-name',
    serviceName,
    '--agent-name',
    agentName,
    '--task-name',
    taskName,
    '--login-id',
    targetServiceId,
    '--target-service-id',
    targetServiceId,
    '--readiness-profile-id',
    profileId,
    '--register-profile-id',
    profileId,
    '--profile-user-data-dir',
    userDataDir,
    '--run-due-readiness-monitor',
    '--url',
    pageUrl,
  ]);
  const output = parseJsonOutput(flowResult.stdout, 'managed profile flow');

  assert(output.dryRun === false, `flow did not run live: ${JSON.stringify(output)}`);
  assert(
    output.initialAccessPlan?.selectedProfile?.id === profileId,
    `flow did not ask broker for the existing profile before fallback registration: ${JSON.stringify(output.initialAccessPlan)}`,
  );
  assert(
    output.initialAccessPlan?.decision?.recommendedAction === 'run_due_profile_readiness_monitor',
    `flow did not see the due monitor recommendation first: ${JSON.stringify(output.initialAccessPlan?.decision)}`,
  );
  assert(
    output.profileRegistration === null,
    `flow registered a profile even though the broker selected one: ${JSON.stringify(output.profileRegistration)}`,
  );
  assert(
    output.monitorRunDue?.checked === 1 && output.monitorRunDue?.failed === 0,
    `flow did not run exactly one successful due monitor: ${JSON.stringify(output.monitorRunDue)}`,
  );
  const expectedAcquisitionSummary = {
    selectedProfileId: profileId,
    registered: false,
    monitorRegistered: false,
    monitorRunDueRan: true,
    initialRecommendedAction: 'run_due_profile_readiness_monitor',
    refreshedRecommendedAction: 'use_selected_profile',
    monitorRunDueChecked: 1,
    monitorRunDueFailed: 0,
  };
  assert(
    JSON.stringify(output.profileAcquisitionSummary) === JSON.stringify(expectedAcquisitionSummary),
    `flow acquisition summary mismatch: ${JSON.stringify(output.profileAcquisitionSummary)}`,
  );
  assert(
    output.accessPlan?.decision?.recommendedAction === 'use_selected_profile',
    `flow did not refresh to the usable profile recommendation: ${JSON.stringify(output.accessPlan?.decision)}`,
  );
  assert(output.tab?.success === true, `flow service tab request failed: ${JSON.stringify(output.tab)}`);

  const state = readServiceState();
  const monitor = state.monitors?.[monitorId];
  assert(monitor?.lastCheckedAt, `due monitor was not persisted as checked: ${JSON.stringify(monitor)}`);
  assert(
    monitor?.lastResult === 'profile_readiness_fresh',
    `due monitor did not record fresh readiness: ${JSON.stringify(monitor)}`,
  );
  assert(
    state.profiles?.[profileId]?.authenticatedServiceIds?.includes(targetServiceId),
    `fresh due monitor should preserve authenticated target: ${JSON.stringify(state.profiles?.[profileId])}`,
  );

  await cleanup();
  console.log('Managed profile flow live smoke passed');
} catch (err) {
  await fail(err.stack || err.message || String(err));
}
