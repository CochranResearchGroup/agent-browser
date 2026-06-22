#!/usr/bin/env node

import { request } from 'node:http';

import {
  assert,
  closeSession,
  createSmokeContext,
} from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';

const context = createSmokeContext({
  prefix: 'ab-service-probe-',
  sessionPrefix: 'service-probe',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'ServiceProbeSmoke';
const agentName = 'smoke-agent';
const taskName = 'plan0034GenericProbe';
const profileId = 'generic-probe-profile';
const targetServiceId = 'generic-probe-site';
const accountId = 'probe-account@example.test';
const browserId = `session:${session}`;
const html = `<!doctype html>
<html>
  <head><title>Plan 0034 Generic Probe</title></head>
  <body>
    <main data-account="${accountId}">Signed in as ${accountId}</main>
    <script>
      window.__genericProbeIdentity = {
        detectedIdentity: "${accountId}",
        accountId: "${accountId}",
        confidence: "high"
      };
    </script>
  </body>
</html>`;
const pageUrl = `data:text/html;charset=utf-8,${encodeURIComponent(html)}`;
const timeout = setTimeout(() => {
  fail('Timed out waiting for service probe live smoke to complete');
}, 180000);

let streamPort;

async function cleanup() {
  clearTimeout(timeout);
  await closeSession(context);
  context.cleanupTempHome();
}

async function fail(message) {
  console.error(message);
  await cleanup();
  process.exit(1);
}

function httpJsonWithTimeout(port, method, path, body, timeoutMs) {
  return new Promise((resolve, reject) => {
    const rawBody = body === undefined ? undefined : JSON.stringify(body);
    const req = request(
      {
        host: '127.0.0.1',
        port,
        method,
        path,
        headers: rawBody
          ? {
              'content-type': 'application/json',
              'content-length': Buffer.byteLength(rawBody),
            }
          : undefined,
      },
      (res) => {
        let text = '';
        res.setEncoding('utf8');
        res.on('data', (chunk) => {
          text += chunk;
        });
        res.on('end', () => {
          try {
            const parsed = JSON.parse(text);
            if (!res.statusCode || res.statusCode < 200 || res.statusCode >= 300) {
              reject(new Error(`HTTP ${method} ${path} returned ${res.statusCode}: ${text}`));
              return;
            }
            resolve(parsed);
          } catch (err) {
            reject(new Error(`Failed to parse HTTP ${method} ${path}: ${err.message}\n${text}`));
          }
        });
      },
    );
    req.setTimeout(timeoutMs, () => {
      req.destroy(new Error(`HTTP ${method} ${path} timed out after ${timeoutMs}ms`));
    });
    req.on('error', reject);
    if (rawBody) req.write(rawBody);
    req.end();
  });
}

async function serviceRequest(body, label) {
  let response;
  try {
    response = await httpJsonWithTimeout(streamPort, 'POST', '/api/service/request', {
      serviceName,
      agentName,
      taskName,
      jobTimeoutMs: 60000,
      ...body,
    }, 90000);
  } catch (err) {
    throw new Error(`${label} failed: ${err.message}`);
  }
  assert(response.success === true, `${label} failed: ${JSON.stringify(response)}`);
  return response;
}

try {
  streamPort = await ensureStreamPort(context, 120000);

  const profileUpsert = await httpJsonWithTimeout(
    streamPort,
    'POST',
    `/api/service/profiles/${encodeURIComponent(profileId)}`,
    {
      name: 'Generic Probe Profile',
      allocation: 'per_service',
      keyring: 'basic_password_store',
      persistent: true,
      targetServiceIds: [targetServiceId],
      accountIds: [accountId],
      sharedServiceIds: [serviceName],
    },
    60000,
  );
  assert(profileUpsert.success === true, `profile upsert failed: ${JSON.stringify(profileUpsert)}`);
  assert(profileUpsert.data?.profile?.id === profileId, `profile id mismatch: ${JSON.stringify(profileUpsert)}`);

  const tabResponse = await serviceRequest(
    {
      action: 'tab_new',
      targetServiceId,
      accountId,
      runtimeProfile: profileId,
      params: {
        headless: true,
        url: pageUrl,
        waitUntil: 'load',
      },
    },
    'tab_new',
  );
  const handle = tabResponse.data?.serviceTabHandle;
  assert(handle?.valid === true, `tab_new did not return a valid serviceTabHandle: ${JSON.stringify(tabResponse)}`);
  assert(handle?.browserId === browserId, `serviceTabHandle browser mismatch: ${JSON.stringify(handle)}`);
  assert(typeof handle?.targetId === 'string' && handle.targetId, `serviceTabHandle missing targetId: ${JSON.stringify(handle)}`);

  const probeResponse = await serviceRequest(
    {
      action: 'probe',
      targetServiceId,
      accountId,
      serviceTabHandle: handle,
      timeoutMs: 3000,
      maxReturnBytes: 512,
      probe: {
        recipeId: 'generic-identity-smoke',
        expectedIdentity: accountId,
        detectors: [
          { id: 'page', type: 'url_title' },
          { id: 'account-text', type: 'selector_text', selector: '[data-account]', maxTextBytes: 128 },
          { id: 'identity-object', type: 'evaluate', expression: 'window.__genericProbeIdentity' },
        ],
        recordFreshness: {
          profileId,
          targetServiceId,
          accountId,
          readinessState: 'fresh',
          readinessRecommendedAction: 'profile_freshness_verified_by_service_probe',
        },
      },
    },
    'probe',
  );
  const probe = probeResponse.data;
  assert(probe?.ok === true, `probe was not ok: ${JSON.stringify(probeResponse)}`);
  assert(probe?.action === 'probe', `probe action mismatch: ${JSON.stringify(probe)}`);
  assert(probe?.identity?.detectedIdentity === accountId, `probe identity mismatch: ${JSON.stringify(probe?.identity)}`);
  assert(probe?.identity?.confidence === 'high', `probe confidence mismatch: ${JSON.stringify(probe?.identity)}`);
  assert(probe?.freshness?.recorded === true, `probe did not record freshness: ${JSON.stringify(probe?.freshness)}`);
  assert(
    probe?.freshness?.profile?.authenticatedServiceIds?.includes(targetServiceId),
    `probe freshness did not authenticate target: ${JSON.stringify(probe?.freshness?.profile)}`,
  );
  assert(
    probe?.freshness?.profile?.accountIds?.includes(accountId),
    `probe freshness did not retain account id: ${JSON.stringify(probe?.freshness?.profile)}`,
  );
  assert(
    probe?.detectors?.some((detector) => detector.id === 'account-text' && detector.ok === true),
    `selector_text detector missing: ${JSON.stringify(probe?.detectors)}`,
  );

  const profiles = await httpJsonWithTimeout(streamPort, 'GET', '/api/service/profiles', undefined, 60000);
  const profile = profiles.data?.profiles?.find((item) => item.id === profileId);
  assert(profile, `profiles collection missing ${profileId}: ${JSON.stringify(profiles)}`);
  assert(
    profile.targetReadiness?.some((row) => row.targetServiceId === targetServiceId && row.state === 'fresh'),
    `profiles collection missing fresh readiness row: ${JSON.stringify(profile)}`,
  );
  const accessPlan = await httpJsonWithTimeout(
    streamPort,
    'GET',
    `/api/service/access-plan?serviceName=${encodeURIComponent(serviceName)}&agentName=${encodeURIComponent(agentName)}&taskName=${encodeURIComponent(taskName)}&targetServiceId=${encodeURIComponent(targetServiceId)}&accountId=${encodeURIComponent(accountId)}`,
    undefined,
    60000,
  );
  assert(accessPlan.success === true, `access-plan readback failed: ${JSON.stringify(accessPlan)}`);
  assert(
    accessPlan.data?.selectedProfile?.id === profileId,
    `access-plan did not select probe profile: ${JSON.stringify(accessPlan.data)}`,
  );
  assert(
    accessPlan.data?.readinessSummary?.freshTargetServiceIds?.includes(targetServiceId) ||
      accessPlan.data?.selectedProfile?.targetReadiness?.some((row) => row.targetServiceId === targetServiceId && row.state === 'fresh'),
    `access-plan did not expose fresh probe readiness: ${JSON.stringify(accessPlan.data)}`,
  );

  await cleanup();
  console.log(`Service probe live smoke passed (${browserId}, stream ${streamPort})`);
} catch (err) {
  await fail(err.stack || err.message);
}
