#!/usr/bin/env node

import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { getServiceAccessPlan } from '../packages/client/src/service-observability.js';
import {
  requestServiceExternalByopAdopt,
} from '../packages/client/src/service-request.js';
import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-external-byop-adopt-',
  sessionPrefix: 'external-byop-adopt',
});

const { agentHome, session, tempHome } = context;
if (!process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD && existsSync('/usr/bin/google-chrome')) {
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = '/usr/bin/google-chrome';
}
const externalSession = `${session}-source`;
const serviceName = 'ExternalByopAdoptSmoke';
const agentName = 'smoke-agent';
const taskName = 'adoptExistingChrome';
const targetServiceId = 'auracall-smoke';
const profileId = `external-byop-smoke-${process.pid}`;
const userDataDir = join(tempHome, 'external-byop-user-data');
const adoptUrl = smokeDataUrl('External BYOP Adopt Smoke', 'External BYOP Adopt Smoke');

const timeout = setTimeout(() => {
  fail('Timed out waiting for external BYOP adopt smoke to complete');
}, 120000);

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
            name: 'External BYOP smoke profile',
            profileOrigin: 'external_byop',
            userDataDir,
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [targetServiceId],
            sharedServiceIds: [serviceName],
            persistent: true,
            cleanup: 'detach',
            registration: {
              serviceName,
              agentName,
              taskName,
            },
          },
        },
      },
      null,
      2,
    )}\n`,
  );
}

async function streamPort() {
  const statusResult = await runCli(context, ['--json', '--session', session, 'stream', 'status']);
  let stream = parseJsonOutput(statusResult.stdout, 'stream status');
  assert(stream.success === true, `stream status failed: ${statusResult.stdout}${statusResult.stderr}`);
  if (!stream.data?.enabled) {
    const streamResult = await runCli(context, ['--json', '--session', session, 'stream', 'enable']);
    stream = parseJsonOutput(streamResult.stdout, 'stream enable');
    assert(stream.success === true, `stream enable failed: ${streamResult.stdout}${streamResult.stderr}`);
  }
  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream did not return a port: ${JSON.stringify(stream)}`);
  return port;
}

async function cleanup() {
  clearTimeout(timeout);
  try {
    await closeSession({ ...context, session: externalSession });
  } catch {
    // Best-effort cleanup for the source browser.
  }
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

try {
  seedServiceState();

  const sourceOpen = await runCli(context, ['--json', '--session', externalSession, 'open', adoptUrl]);
  const sourceOpenJson = parseJsonOutput(sourceOpen.stdout, 'source open');
  assert(sourceOpenJson.success === true, `source open failed: ${sourceOpen.stdout}${sourceOpen.stderr}`);

  const cdpResult = await runCli(context, ['--json', '--session', externalSession, 'get', 'cdp-url']);
  const cdp = parseJsonOutput(cdpResult.stdout, 'source cdp-url');
  const cdpUrl = cdp.data?.cdpUrl;
  assert(typeof cdpUrl === 'string' && cdpUrl.startsWith('ws://'), `source cdp-url missing: ${cdpResult.stdout}`);

  const port = await streamPort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const adopted = await requestServiceExternalByopAdopt({
    baseUrl,
    serviceName,
    agentName,
    taskName,
    runtimeProfile: profileId,
    cdpUrl,
    url: adoptUrl,
  });
  assert(adopted.success === true, `adoption request failed: ${JSON.stringify(adopted)}`);
  assert(adopted.data?.adopted === true, `adoption did not report adopted: ${JSON.stringify(adopted)}`);
  assert(
    adopted.data?.serviceTabHandle?.profileOrigin === 'external_byop',
    `adoption returned wrong profile origin: ${JSON.stringify(adopted.data?.serviceTabHandle)}`,
  );

  const plan = await getServiceAccessPlan({
    baseUrl,
    serviceName,
    agentName,
    taskName: 'verifyReuseAfterAdopt',
    targetServiceId,
  });
  assert(
    plan.decision?.profileReuse?.recommendedAction === 'reuse_existing_browser',
    `access plan did not recommend reuse: ${JSON.stringify(plan.decision?.profileReuse)}`,
  );
  assert(
    plan.decision?.profileReuse?.reusableBrowserId === `session:${session}`,
    `access plan reused wrong browser: ${JSON.stringify(plan.decision?.profileReuse)}`,
  );
  assert(
    plan.decision?.serviceRequest?.request?.sessionName === session,
    `access plan did not include reusable session route: ${JSON.stringify(plan.decision?.serviceRequest)}`,
  );

  const browsers = await httpJson(port, 'GET', '/api/service/browsers');
  assert(browsers.success === true, `browser readback failed: ${JSON.stringify(browsers)}`);
  const adoptedBrowser = browsers.data?.browsers?.find((browser) => browser.id === `session:${session}`);
  assert(adoptedBrowser, `adopted browser missing from readback: ${JSON.stringify(browsers.data)}`);
  assert(adoptedBrowser.profileId === profileId, `adopted browser profile mismatch: ${JSON.stringify(adoptedBrowser)}`);
  assert(adoptedBrowser.host === 'attached_existing', `adopted browser host mismatch: ${JSON.stringify(adoptedBrowser)}`);

  await cleanup();
  console.log('External BYOP adopt live smoke passed');
} catch (err) {
  await fail(err instanceof Error ? err.stack || err.message : String(err));
}
