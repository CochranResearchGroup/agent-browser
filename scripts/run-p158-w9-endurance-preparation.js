#!/usr/bin/env node

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { sha256 } from './lib/p158-campaign-controller.js';
import { prepareP158W9EndurancePostconditions } from './lib/p158-w9-endurance-preparation.js';

function take(args, flag) {
  const index = args.indexOf(flag);
  if (index < 0 || index + 1 >= args.length) throw new Error(`${flag} requires a value`);
  const value = args[index + 1];
  args.splice(index, 2);
  return value;
}

function safeMessage(error, env) {
  let value = error?.message ?? String(error);
  for (const secret of [env.P158_DEV_HANDOFF_URL, env.P158_DEV_DASHBOARD_USERNAME,
    env.P158_DEV_DASHBOARD_PASSWORD, env.P158_DEV_EXPECTED_IDENTITY_JSON]) {
    if (secret) value = value.split(secret).join('[redacted]');
  }
  return value.slice(0, 1000);
}

function artifactStore(root) {
  return { writeArtifact: async ({ artifactId, relativePath, content }) => {
    const path = resolve(root, relativePath);
    if (!path.startsWith(`${resolve(root)}/`)) throw new Error('Artifact path escaped preparation root');
    mkdirSync(dirname(path), { recursive: true, mode: 0o700 });
    writeFileSync(path, content, { mode: 0o600, flag: 'wx' });
    return { artifactId, relativePath, sha256: sha256(content), byteCount: content.byteLength };
  } };
}

function resolutionReady(resolution) {
  const receipt = resolution?.presentationReceipt;
  return resolution?.resolved === true && resolution.status === 'ready' &&
    Number.isInteger(resolution.presentationGeneration) && resolution.presentationGeneration > 0 &&
    receipt?.generation === resolution.presentationGeneration && receipt.state === 'ready' &&
    receipt.logicalBrowserId === resolution.browserId && receipt.targetId === resolution.targetId;
}

function retainedIdentity(resolution) {
  const tab = resolution.tab ?? {};
  const handle = tab.serviceTabHandle ?? {};
  const intent = resolution.open?.intent ?? {};
  return {
    browserId: resolution.browserId ?? tab.browserId ?? handle.browserId,
    profileId: tab.profileId ?? tab.runtimeProfile ?? handle.profileId ?? intent.runtimeProfile ?? intent.profile,
    sessionId: resolution.sessionName ?? tab.sessionId ?? handle.sessionName,
    tabId: resolution.tabId ?? tab.tabId ?? tab.id ?? handle.tabId,
    targetId: resolution.targetId ?? tab.targetId ?? handle.targetId,
    visibleUrl: tab.url ?? intent.url,
    pageMarker: tab.pageMarker ?? tab.title,
  };
}

async function createPlaywrightBrowser({ config, env }) {
  const [{ chromium }, { projectHandoffResolution }] = await Promise.all([
    import('playwright'), import('./run-p158-external-vantage.js'),
  ]);
  let browser;
  let context;
  let page;
  let resolutions = [];
  const runnerIdentitySha256 = sha256({
    workflowRunId: env.GITHUB_RUN_ID, workflowRunAttempt: Number(env.GITHUB_RUN_ATTEMPT),
    workflowJob: env.GITHUB_JOB, runnerName: env.RUNNER_NAME, runnerOs: env.RUNNER_OS, runnerArch: env.RUNNER_ARCH,
  });
  if (runnerIdentitySha256 !== config.externalRunnerIdentitySha256) {
    throw new Error('External runner identity differs from the frozen preparation input');
  }
  if (config.workflowRunId !== env.GITHUB_RUN_ID || config.workflowRunAttempt !== Number(env.GITHUB_RUN_ATTEMPT) ||
      config.workflowJob !== env.GITHUB_JOB || config.sourceCommit !== env.GITHUB_SHA) {
    throw new Error('Preparation workflow or source identity differs from the bound input');
  }

  async function start() {
    browser = await chromium.launch({ headless: true });
    context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, reducedMotion: 'reduce' });
    const response = await context.request.post(new URL('/api/dashboard-auth/login', env.P158_DEV_HANDOFF_URL).href, {
      data: { username: env.P158_DEV_DASHBOARD_USERNAME, password: env.P158_DEV_DASHBOARD_PASSWORD },
      failOnStatusCode: false,
    });
    if (!response.ok()) throw new Error(`Dashboard authentication failed with HTTP ${response.status()}`);
  }

  async function stop() {
    await page?.close().catch(() => {});
    await context?.close().catch(() => {});
    await browser?.close().catch(() => {});
    page = undefined; context = undefined; browser = undefined;
  }

  async function openHandoff() {
    if (!browser) await start();
    await page?.close().catch(() => {});
    page = await context.newPage();
    const startIndex = resolutions.length;
    page.on('response', async (response) => {
      let path;
      try { path = new URL(response.url()).pathname; } catch { return; }
      if (path !== '/api/service/request' || response.request().method() !== 'POST' ||
          !(response.headers()['content-type'] ?? '').includes('application/json')) return;
      let request;
      try { request = response.request().postDataJSON(); } catch { return; }
      if (request?.action !== 'service_remote_view_handoff_resolve') return;
      const envelope = await response.json().catch(() => null);
      if (envelope?.success === true && envelope.data) resolutions.push(projectHandoffResolution(envelope.data));
    });
    const navigation = await page.goto(env.P158_DEV_HANDOFF_URL, { waitUntil: 'domcontentloaded', timeout: 45_000 });
    if (!navigation?.ok()) throw new Error('Durable handoff navigation failed');
    const deadline = Date.now() + 30_000;
    let resolution;
    while (Date.now() < deadline && !resolution) {
      resolution = resolutions.slice(startIndex).find(resolutionReady);
      if (!resolution) await new Promise((done) => setTimeout(done, 200));
    }
    if (!resolution) throw new Error('Authoritative operatorVisible ready was not observed');
    const readyObservedAt = new Date().toISOString();
    const pixelBytes = await page.screenshot({ clip: JSON.parse(env.P158_DEV_PIXEL_MARKER_REGION_JSON) });
    const pixelsObservedAt = new Date().toISOString();
    const expectedIdentity = JSON.parse(env.P158_DEV_EXPECTED_IDENTITY_JSON);
    const observedIdentity = retainedIdentity(resolution);
    for (const field of Object.keys(observedIdentity)) {
      if (observedIdentity[field] !== expectedIdentity[field]) {
        throw new Error(`Resolved browser identity differs on ${field}`);
      }
    }
    if (sha256(expectedIdentity) !== config.retainedIdentitySha256 || sha256(pixelBytes) !== expectedIdentity.pixelHash) {
      throw new Error('Resolved browser identity differs from the frozen identity digest');
    }
    return {
      operatorVisibleState: 'ready', readyBeforePixels: true, readyObservedAt, pixelsObservedAt,
      handoffUrlSha256: config.handoffUrlSha256, retainedIdentitySha256: config.retainedIdentitySha256,
      candidateSha256: config.candidateSha256, scheduleSha256: config.scheduleSha256, runId: config.runId,
      offHost: true, outsideServiceHost: true, outsideServiceNetworkNamespace: true,
      externalRunnerIdentitySha256: runnerIdentitySha256,
      pixelBytes,
    };
  }

  return {
    openHandoff,
    resetSyntheticFixture: openHandoff,
    captureRegion: async ({ region }) => page.screenshot({ clip: region }),
    performDashboardAction: async ({ action, interaction }) => {
      if (interaction.kind !== 'remote_pixel_click' || !Number.isFinite(interaction.x) || !Number.isFinite(interaction.y)) {
        throw new Error(`${action.actionId} lacks reviewed remote pixel coordinates`);
      }
      await page.mouse.click(interaction.x, interaction.y);
      if (Number.isInteger(interaction.settleMs) && interaction.settleMs > 0) {
        await page.waitForTimeout(Math.min(interaction.settleMs, 5000));
      }
      return { actionId: action.actionId, observed: true };
    },
    readViewerLeases: async () => {
      const response = await context.request.get(new URL('/api/service/viewer-leases', env.P158_DEV_HANDOFF_URL).href,
        { failOnStatusCode: false });
      if (!response.ok() || !(response.headers()['content-type'] ?? '').includes('application/json')) {
        throw new Error('Viewer lease baseline request failed');
      }
      const body = await response.json();
      return (Array.isArray(body?.viewerLeases) ? body.viewerLeases : []).map((lease) => ({
        id: lease.id ?? lease.viewerLeaseId, viewerRole: lease.viewerRole ?? lease.role,
        state: lease.state, generation: lease.generation ?? lease.leaseGeneration,
      }));
    },
    probeNetworkRecovery: async () => {
      await context.setOffline(true);
      let offlineFailureObserved = false;
      try { await page.goto(env.P158_DEV_HANDOFF_URL, { waitUntil: 'domcontentloaded', timeout: 10_000 }); }
      catch { offlineFailureObserved = true; }
      finally { await context.setOffline(false); }
      const proof = await openHandoff();
      return { ...proof, offlineFailureObserved };
    },
    probeClientRestart: async () => {
      await stop();
      resolutions = [];
      const proof = await openHandoff();
      return { ...proof, clientRestartObserved: true };
    },
    close: stop,
  };
}

async function main(args, env) {
  const inputPath = resolve(take(args, '--input'));
  const outputRoot = resolve(take(args, '--output-dir'));
  if (args.length) throw new Error(`Unexpected arguments: ${args.join(' ')}`);
  mkdirSync(outputRoot, { recursive: true, mode: 0o700 });
  let browser;
  let input;
  try {
    input = JSON.parse(readFileSync(inputPath, 'utf8'));
    const config = { ...input.config, handoffUrl: env.P158_DEV_HANDOFF_URL };
    browser = await createPlaywrightBrowser({ config, env });
    const result = await prepareP158W9EndurancePostconditions({
      config, actions: input.actions, dashboardProbes: input.dashboardProbes,
      browser, artifactStore: artifactStore(outputRoot),
    });
    writeFileSync(join(outputRoot, 'postcondition-preparation.json'), `${JSON.stringify(result, null, 2)}\n`,
      { mode: 0o600, flag: 'wx' });
  } catch (error) {
    writeFileSync(join(outputRoot, 'failure-receipt.json'), `${JSON.stringify({
      schemaVersion: 'agent-browser.p158-w9-endurance-postcondition-preparation-failure.v1', planId: 'P158',
      runId: input?.config?.runId ?? null, caseId: input?.config?.caseId ?? null,
      failedAt: new Date().toISOString(), failure: { code: error.code ?? 'preparation_failed', message: safeMessage(error, env) },
      retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
    }, null, 2)}\n`, { mode: 0o600, flag: 'wx' });
    throw error;
  } finally {
    await browser?.close();
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main(process.argv.slice(2), process.env);
}
