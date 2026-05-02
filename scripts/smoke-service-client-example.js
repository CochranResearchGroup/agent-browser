#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
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
  prefix: 'ab-service-client-example-',
  sessionPrefix: 'service-client-example',
});

const { agentHome, session, tempHome } = context;
const serviceName = 'JournalDownloader';
const agentName = 'article-probe-agent';
const taskName = 'probeACSwebsite';
const targetServiceId = 'example';
const profileId = `service-client-example-profile-${process.pid}`;
const userDataDir = join(tempHome, 'service-client-example-user-data');

const timeout = setTimeout(() => {
  fail('Timed out waiting for service client example smoke to complete');
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
            name: 'Service client example profile',
            userDataDir,
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [targetServiceId],
            sharedServiceIds: [serviceName],
            persistent: true,
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

function runExample(args) {
  return new Promise((resolve, reject) => {
    const proc = spawn(process.execPath, ['examples/service-client/service-request-trace.mjs', ...args], {
      cwd: rootDir,
      env: context.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const procTimeout = setTimeout(() => {
      proc.kill('SIGTERM');
      reject(new Error(`service client example timed out: ${args.join(' ')}`));
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
        reject(
          new Error(
            `service client example failed: code=${code} signal=${signal}\n${out}${err}`,
          ),
        );
      }
    });
  });
}

try {
  seedServiceState();

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status');
  assert(status.success === true, `service status failed: ${statusResult.stdout}${statusResult.stderr}`);

  const port = await ensureStreamPort();
  const pageUrl = smokeDataUrl('Service Client Example Smoke', 'Service Client Example Smoke');
  const exampleResult = await runExample([
    '--base-url',
    `http://127.0.0.1:${port}`,
    '--url',
    pageUrl,
    '--service-name',
    serviceName,
    '--agent-name',
    agentName,
    '--task-name',
    taskName,
    '--site-id',
    targetServiceId,
    '--login-id',
    targetServiceId,
  ]);
  const output = parseJsonOutput(exampleResult.stdout, 'service client example');

  assert(output.dryRun === false, `example did not run live: ${JSON.stringify(output)}`);
  assert(
    output.commandResult?.success === true,
    `example service tab request failed: ${JSON.stringify(output.commandResult)}`,
  );
  assert(output.traceSummary?.jobs >= 1, `example trace summary missed jobs: ${JSON.stringify(output)}`);
  const latestJob = output.latestJobs?.find(
    (job) =>
      job.action === 'tab_new' &&
      job.state === 'succeeded' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(latestJob, `example trace did not include the succeeded named job: ${JSON.stringify(output.latestJobs)}`);

  await cleanup();
  console.log('Service client example live smoke passed');
} catch (err) {
  await fail(err.stack || err.message || String(err));
}
