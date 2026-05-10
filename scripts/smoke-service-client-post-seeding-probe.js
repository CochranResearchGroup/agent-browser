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
  prefix: 'ab-post-seeding-probe-',
  sessionPrefix: 'post-seeding-probe',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { agentHome, session, tempHome } = context;
const serviceName = 'PostSeedingProbeSmoke';
const agentName = 'post-seeding-probe-agent';
const taskName = 'verifySeededProfile';
const targetServiceId = 'post-seeding-example';
const profileId = `post-seeding-probe-profile-${process.pid}`;
const handoffId = `${profileId}:${targetServiceId}`;
const userDataDir = join(tempHome, 'post-seeding-probe-user-data');

const timeout = setTimeout(() => {
  fail('Timed out waiting for post-seeding probe smoke to complete');
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
            name: 'Post-seeding probe smoke profile',
            userDataDir,
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [targetServiceId],
            sharedServiceIds: [serviceName],
            targetReadiness: [
              {
                targetServiceId,
                loginId: targetServiceId,
                state: 'seeded_unknown_freshness',
                manualSeedingRequired: false,
                evidence: 'seeding_browser_closed_unverified',
                recommendedAction: 'verify_profile_freshness',
                seedingMode: 'detached_headed_no_cdp',
                cdpAttachmentAllowedDuringSeeding: false,
                preferredKeyring: 'basic_password_store',
                setupScopes: ['signin'],
                lastVerifiedAt: null,
                freshnessExpiresAt: null,
              },
            ],
            persistent: true,
          },
        },
        profileSeedingHandoffs: {
          [handoffId]: {
            id: handoffId,
            profileId,
            targetServiceId,
            state: 'seeding_closed_unverified',
            pid: null,
            startedAt: '2026-05-10T12:00:00Z',
            expiresAt: null,
            lastPromptedAt: null,
            declaredCompleteAt: null,
            closedAt: '2026-05-10T12:05:00Z',
            updatedAt: '2026-05-10T12:05:00Z',
            actor: 'smoke',
            note: 'Seeded by live post-seeding probe smoke',
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

function runProbe(args) {
  return new Promise((resolve, reject) => {
    const proc = spawn(process.execPath, ['examples/service-client/post-seeding-probe.mjs', ...args], {
      cwd: rootDir,
      env: context.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const procTimeout = setTimeout(() => {
      proc.kill('SIGTERM');
      reject(new Error(`post-seeding probe timed out: ${args.join(' ')}`));
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
        reject(new Error(`post-seeding probe failed: code=${code} signal=${signal}\n${out}${err}`));
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
  const pageUrl = smokeDataUrl('Post Seeding Probe Smoke', 'Post Seeding Probe Smoke');
  const probeResult = await runProbe([
    '--base-url',
    `http://127.0.0.1:${port}`,
    '--profile-id',
    profileId,
    '--url',
    pageUrl,
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
    '--expected-title-includes',
    'Post Seeding Probe Smoke',
  ]);
  const output = parseJsonOutput(probeResult.stdout, 'post-seeding probe');

  assert(output.dryRun === false, `probe did not run live: ${JSON.stringify(output)}`);
  assert(
    output.lookup?.selectedProfile?.id === profileId,
    `probe did not confirm the broker-selected profile: ${JSON.stringify(output.lookup)}`,
  );
  assert(output.checks?.fresh === true, `probe checks were not fresh: ${JSON.stringify(output.checks)}`);
  assert(
    output.freshness?.profile?.targetReadiness?.some(
      (row) => row.targetServiceId === targetServiceId && row.state === 'fresh',
    ),
    `probe did not record fresh target readiness: ${JSON.stringify(output.freshness)}`,
  );
  assert(
    output.freshness?.profile?.authenticatedServiceIds?.includes(targetServiceId),
    `probe did not preserve authenticated target: ${JSON.stringify(output.freshness)}`,
  );

  const state = readServiceState();
  assert(
    state.profileSeedingHandoffs?.[handoffId]?.state === 'fresh',
    `closed handoff did not advance to fresh: ${JSON.stringify(state.profileSeedingHandoffs?.[handoffId])}`,
  );

  await cleanup();
  console.log('Post-seeding probe live smoke passed');
} catch (err) {
  await fail(err.stack || err.message || String(err));
}
