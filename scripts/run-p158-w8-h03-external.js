#!/usr/bin/env node

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { sha256 } from './lib/p158-campaign-controller.js';
import {
  executeP158W8H03ExternalManifest,
  validateP158W8H03ExternalResult,
} from './lib/p158-w8-h03-external.js';

function take(args, flag) {
  const index = args.indexOf(flag);
  if (index < 0 || index + 1 >= args.length) throw new Error(`${flag} requires a value`);
  const value = args[index + 1]; args.splice(index, 2); return value;
}

function safeMessage(error, env) {
  let value = error?.message ?? String(error);
  for (const secret of [env.P158_DEV_HANDOFF_URL, env.P158_DEV_DASHBOARD_USERNAME,
    env.P158_DEV_DASHBOARD_PASSWORD, env.P158_DEV_EXPECTED_IDENTITY_JSON]) {
    if (secret) value = value.split(secret).join('[redacted]');
  }
  return value.slice(0, 1000);
}

function store(root) {
  return { writeArtifact: async ({ artifactId, relativePath, content }) => {
    const path = resolve(root, relativePath);
    if (!path.startsWith(`${resolve(root)}/`)) throw new Error('Artifact path escaped H03 root');
    mkdirSync(dirname(path), { recursive: true, mode: 0o700 });
    writeFileSync(path, content, { mode: 0o600, flag: 'wx' });
    return { artifactId, relativePath, sha256: sha256(content), byteCount: content.byteLength };
  } };
}

function githubRunnerIdentity(env) {
  if (env.GITHUB_ACTIONS !== 'true' || env.RUNNER_ENVIRONMENT !== 'github-hosted' ||
      env.RUNNER_OS !== 'Linux' || !env.GITHUB_RUN_ID || !env.GITHUB_RUN_ATTEMPT ||
      !env.GITHUB_JOB || !env.RUNNER_NAME || !env.RUNNER_ARCH) {
    throw new Error('H03 execution requires an attested GitHub-hosted Linux runner');
  }
  return sha256({ provider: 'github_actions', environment: env.RUNNER_ENVIRONMENT,
    os: env.RUNNER_OS, architecture: env.RUNNER_ARCH, runId: env.GITHUB_RUN_ID,
    runAttempt: env.GITHUB_RUN_ATTEMPT, job: env.GITHUB_JOB, runnerName: env.RUNNER_NAME });
}

async function createDriver({ manifest, env }) {
  const runnerIdentitySha256 = githubRunnerIdentity(env);
  const [{ chromium }, { projectHandoffResolution }] = await Promise.all([
    import('playwright'), import('./run-p158-external-vantage.js'),
  ]);
  const handoff = new URL(env.P158_DEV_HANDOFF_URL);
  if (sha256(handoff.href) !== manifest.handoffUrlSha256 || sha256(handoff.origin) !== manifest.externalIngress.publicOriginSha256) {
    throw new Error('H03 handoff or public origin differs from the frozen manifest');
  }
  const expectedIdentity = JSON.parse(env.P158_DEV_EXPECTED_IDENTITY_JSON);
  const markerRegion = JSON.parse(env.P158_DEV_PIXEL_MARKER_REGION_JSON);
  let browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, reducedMotion: 'reduce' });
  const auth = await context.request.post(new URL('/api/dashboard-auth/login', handoff.origin).href, {
    data: { username: env.P158_DEV_DASHBOARD_USERNAME, password: env.P158_DEV_DASHBOARD_PASSWORD },
    failOnStatusCode: false,
  });
  if (!auth.ok()) throw new Error(`Dashboard authentication failed with HTTP ${auth.status()}`);
  let page;
  let lastRawResolution;
  let lastWebSocketUrl;

  async function open() {
    await page?.close().catch(() => {});
    page = await context.newPage();
    lastRawResolution = null;
    lastWebSocketUrl = null;
    page.on('websocket', (socket) => {
      let endpoint;
      try { endpoint = new URL(socket.url()); } catch { return; }
      if (endpoint.protocol === 'wss:' && endpoint.hostname === handoff.hostname) lastWebSocketUrl = endpoint.href;
    });
    page.on('response', async (response) => {
      let path;
      try { path = new URL(response.url()).pathname; } catch { return; }
      if (path !== '/api/service/request' || response.request().method() !== 'POST' ||
          !(response.headers()['content-type'] ?? '').includes('application/json')) return;
      let request;
      try { request = response.request().postDataJSON(); } catch { return; }
      if (request?.action !== 'service_remote_view_handoff_resolve') return;
      const envelope = await response.json().catch(() => null);
      if (envelope?.success === true && envelope.data) lastRawResolution = envelope.data;
    });
    const response = await page.goto(handoff.href, { waitUntil: 'domcontentloaded', timeout: 45_000 });
    if (!response?.ok()) throw new Error('H03 durable handoff navigation failed');
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline) {
      const projected = lastRawResolution && projectHandoffResolution(lastRawResolution);
      if (projected?.resolved === true && projected.status === 'ready' &&
          projected.presentationReceipt?.state === 'ready' && lastWebSocketUrl) break;
      await new Promise((done) => setTimeout(done, 200));
    }
    if (!lastRawResolution || !lastWebSocketUrl) {
      throw new Error('H03 authoritative handoff resolution and public WSS transport were not ready');
    }
  }

  function identity(raw) {
    const projected = projectHandoffResolution(raw);
    const tab = projected.tab ?? {};
    const handle = tab.serviceTabHandle ?? {};
    return {
      browserId: projected.browserId ?? tab.browserId ?? handle.browserId,
      profileId: tab.profileId ?? tab.runtimeProfile ?? handle.profileId,
      sessionId: projected.sessionName ?? tab.sessionId ?? handle.sessionName,
      tabId: projected.tabId ?? tab.tabId ?? tab.id ?? handle.tabId,
      targetId: projected.targetId ?? tab.targetId ?? handle.targetId,
      visibleUrl: tab.url, pageMarker: tab.pageMarker ?? tab.title,
    };
  }

  async function captureContinuity() {
    await open();
    const readyObservedAt = new Date().toISOString();
    const pixelBytes = await page.screenshot({ clip: markerRegion });
    const pixelsObservedAt = new Date().toISOString();
    const observedIdentity = identity(lastRawResolution);
    for (const [field, value] of Object.entries(observedIdentity)) {
      if (expectedIdentity[field] !== value) throw new Error(`H03 retained identity mismatch: ${field}`);
    }
    if (sha256(expectedIdentity) !== manifest.retainedIdentitySha256 || sha256(pixelBytes) !== expectedIdentity.pixelHash) {
      throw new Error('H03 retained identity or pixel digest mismatch');
    }
    const route = lastRawResolution.routeBinding ?? lastRawResolution.open?.routeBinding ?? {};
    const leaseResponse = await context.request.get(new URL('/api/service/viewer-leases', handoff.origin).href,
      { failOnStatusCode: false });
    const leaseBody = leaseResponse.ok() ? await leaseResponse.json() : {};
    const leases = Array.isArray(leaseBody.viewerLeases) ? leaseBody.viewerLeases : [];
    const lease = leases.find((entry) => entry.browserId === observedIdentity.browserId && entry.state === 'active') ?? {};
    return {
      ...Object.fromEntries(Object.entries(observedIdentity).slice(0, 5).map(([field, value]) => [`${field}Sha256`, sha256(value)])),
      routeIdSha256: sha256(route.routeId ?? route.id ?? ''),
      displayAllocationIdSha256: sha256(route.displayAllocationId ?? ''),
      connectionIdSha256: sha256(route.connectionId ?? route.routeDescriptor?.connectionId ?? ''),
      viewerLeaseIdSha256: sha256(lease.id ?? lease.viewerLeaseId ?? ''),
      presentationGeneration: lastRawResolution.presentationGeneration,
      operatorVisibleState: 'ready', readyBeforePixels: true, readyObservedAt, pixelsObservedAt,
      offHost: true, outsideServiceHost: true, outsideServiceNetworkNamespace: true,
      runnerAttestationSha256: manifest.externalIngress.runnerAttestationSha256,
      runnerIdentitySha256,
      websocketEndpointSha256: sha256(lastWebSocketUrl),
      handoffUrlSha256: manifest.handoffUrlSha256, retainedIdentitySha256: manifest.retainedIdentitySha256,
      browserLaunchCount: 1, pixelBytes,
    };
  }

  async function applyTransition({ action }) {
    if (action.transition === 'viewer_expiry') {
      await page?.close();
      page = undefined;
      const deadline = Date.now() + action.binding.timeoutMs;
      let observed = false;
      while (Date.now() < deadline) {
        const response = await context.request.get(new URL('/api/service/viewer-leases', handoff.origin).href,
          { failOnStatusCode: false });
        const body = response.ok() ? await response.json() : {};
        const lease = (body.viewerLeases ?? []).find((entry) =>
          sha256(entry.id ?? entry.viewerLeaseId ?? '') === action.binding.viewerLeaseIdSha256);
        if (lease?.state === 'expired') {
          observed = true; break;
        }
        await new Promise((done) => setTimeout(done, 1000));
      }
      return { actionId: action.actionId, observed, requestAttemptCount: 1,
        retryAttempted: false, repairAttempted: false };
    }
    const response = await context.request.post(new URL('/api/service/request', handoff.origin).href, {
      data: action.binding.request, failOnStatusCode: false,
    });
    const contentType = response.headers()['content-type'] ?? '';
    const envelope = contentType.includes('application/json') ? await response.json() : {};
    return { actionId: action.actionId, observed: response.ok() && envelope.success === true,
      requestAttemptCount: 1, retryAttempted: false, repairAttempted: false };
  }

  return { captureContinuity, applyTransition, close: async () => {
    await page?.close().catch(() => {}); await context.close().catch(() => {}); await browser.close().catch(() => {});
  } };
}

async function main(args, env) {
  const manifestPath = resolve(take(args, '--manifest'));
  const outputDir = resolve(take(args, '--output-dir'));
  if (args.length) throw new Error(`Unexpected arguments: ${args.join(' ')}`);
  mkdirSync(outputDir, { recursive: true, mode: 0o700 });
  let driver;
  let manifest;
  try {
    manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
    if (manifest.sourceCommit !== env.GITHUB_SHA) throw new Error('H03 source commit differs from workflow checkout');
    driver = await createDriver({ manifest, env });
    const result = await executeP158W8H03ExternalManifest({ manifest, driver, artifactStore: store(outputDir),
      clock: { wallNow: () => new Date().toISOString() } });
    validateP158W8H03ExternalResult({ result, manifest });
    writeFileSync(join(outputDir, 'h03-result.json'), `${JSON.stringify(result, null, 2)}\n`, { mode: 0o600, flag: 'wx' });
  } catch (error) {
    writeFileSync(join(outputDir, 'failure-receipt.json'), `${JSON.stringify({
      schemaVersion: 'agent-browser.p158-w8-h03-external-failure.v1', planId: 'P158',
      manifestSha256: manifest?.manifestSha256 ?? null, failedAt: new Date().toISOString(),
      failure: { code: error.code ?? 'h03_external_failed', message: safeMessage(error, env) },
      retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
    }, null, 2)}\n`, { mode: 0o600, flag: 'wx' });
    throw error;
  } finally { await driver?.close(); }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) await main(process.argv.slice(2), process.env);
