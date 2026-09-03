#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import {
  bindP158W9EnduranceDispatchTemplate,
  finalizeP158W9Endurance,
  projectP158W9EnduranceActionReceipts,
  runP158W9EnduranceShard,
  validateP158W9EnduranceDispatch,
} from './lib/p158-w9-endurance.js';
import { sha256 } from './lib/p158-campaign-controller.js';

function hashBuffer(value) {
  return createHash('sha256').update(value).digest('hex');
}

function hashText(value) {
  return hashBuffer(Buffer.from(value));
}

function take(args, flag, { multiple = false } = {}) {
  const values = [];
  for (let index = 0; index < args.length;) {
    if (args[index] !== flag) { index += 1; continue; }
    if (index + 1 >= args.length) throw new Error(`${flag} requires a value`);
    values.push(args[index + 1]);
    args.splice(index, 2);
  }
  return multiple ? values : values.at(-1);
}

function artifact(path, artifactId) {
  const bytes = readFileSync(path);
  return { artifactId, relativePath: basename(path), sha256: hashBuffer(bytes), byteCount: bytes.byteLength };
}

function safeMessage(error, env) {
  let value = error?.message ?? String(error);
  for (const secret of [env.P158_DEV_HANDOFF_URL, env.P158_DEV_DASHBOARD_USERNAME,
    env.P158_DEV_DASHBOARD_PASSWORD, env.P158_DEV_EXPECTED_IDENTITY_JSON]) {
    if (secret) value = value.split(secret).join('[redacted]');
  }
  return value.slice(0, 1000);
}

function resolutionReady(resolution) {
  const receipt = resolution?.presentationReceipt;
  return resolution?.resolved === true && resolution.status === 'ready' &&
    Number.isInteger(resolution.presentationGeneration) && resolution.presentationGeneration > 0 &&
    receipt?.generation === resolution.presentationGeneration &&
    receipt.logicalBrowserId === resolution.browserId && receipt.targetId === resolution.targetId &&
    receipt.requiredStreamProvider === resolution.viewStreamProvider &&
    receipt.observedStreamProvider === receipt.requiredStreamProvider && receipt.state === 'ready';
}

function exactIdentity(resolution, expected) {
  const tab = resolution.tab ?? {};
  const handle = tab.serviceTabHandle ?? {};
  const intent = resolution.open?.intent ?? {};
  const observed = {
    browserId: resolution.browserId ?? tab.browserId ?? handle.browserId,
    profileId: tab.profileId ?? tab.runtimeProfile ?? handle.profileId ?? intent.runtimeProfile ?? intent.profile,
    sessionId: resolution.sessionName ?? tab.sessionId ?? handle.sessionName,
    tabId: resolution.tabId ?? tab.tabId ?? tab.id ?? handle.tabId,
    targetId: resolution.targetId ?? tab.targetId ?? handle.targetId,
    visibleUrl: tab.url ?? intent.url,
    pageMarker: tab.pageMarker ?? tab.title,
  };
  for (const field of Object.keys(observed)) {
    if (observed[field] !== expected[field]) throw new Error(`Retained identity mismatch: ${field}`);
  }
  return observed;
}

async function waitForResolution(resolutions, startIndex, timeoutMs = 30_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const resolution = resolutions.slice(startIndex).find(resolutionReady);
    if (resolution) return resolution;
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 200));
  }
  throw new Error('Authoritative handoff resolution was not ready before pixels');
}

function attachResolverCapture(page, resolutions, projectHandoffResolution) {
  page.on('response', async (response) => {
    const request = response.request();
    let path;
    try { path = new URL(response.url()).pathname; } catch { return; }
    if (path !== '/api/service/request' || request.method() !== 'POST' ||
        !(response.headers()['content-type'] ?? '').includes('application/json')) return;
    let payload;
    try { payload = request.postDataJSON(); } catch { return; }
    if (payload?.action !== 'service_remote_view_handoff_resolve') return;
    const envelope = await response.json().catch(() => null);
    if (envelope?.success === true && envelope.data) resolutions.push(projectHandoffResolution(envelope.data));
  });
}

async function authenticate(context, handoff, env) {
  const response = await context.request.post(new URL('/api/dashboard-auth/login', handoff.origin).href, {
    data: { username: env.P158_DEV_DASHBOARD_USERNAME, password: env.P158_DEV_DASHBOARD_PASSWORD },
    failOnStatusCode: false,
  });
  if (!response.ok()) throw new Error(`Dashboard authentication failed with HTTP ${response.status()}`);
}

async function createLiveDriver({ dispatch, outputDir, env }) {
  const [{ chromium }, { projectHandoffResolution, validateExternalVantageConfiguration }] = await Promise.all([
    import('playwright'),
    import('./run-p158-external-vantage.js'),
  ]);
  const configuration = validateExternalVantageConfiguration({
    env, clientId: `external-runner-${dispatch.caseId.toLowerCase()}-${env.P158_SEGMENT_INDEX}`,
    paceProfile: 'slow_concurrency', mode: 'readiness',
  });
  if (hashText(configuration.handoff.href) !== dispatch.handoffUrlSha256 ||
      env.GITHUB_RUN_ID !== dispatch.workflowRunId || Number(env.GITHUB_RUN_ATTEMPT) !== dispatch.workflowRunAttempt ||
      env.GITHUB_SHA !== dispatch.sourceCommit || env.P158_RUN_ID !== dispatch.runId) {
    throw new Error('Live runner identity, workflow, commit, or handoff differs from the frozen dispatch');
  }
  let browser = await chromium.launch({ headless: true });
  let context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, reducedMotion: 'reduce' });
  await authenticate(context, configuration.handoff, env);
  let page = null;
  let resolutions = [];
  let screenshotOrdinal = 0;

  async function closePage() {
    await page?.close().catch(() => {});
    page = null;
  }

  async function openAndVerify(label) {
    await closePage();
    page = await context.newPage();
    attachResolverCapture(page, resolutions, projectHandoffResolution);
    const startIndex = resolutions.length;
    const response = await page.goto(configuration.handoff.href, {
      waitUntil: 'domcontentloaded', timeout: 45_000,
    });
    if (!response?.ok()) throw new Error(`${label} handoff navigation failed`);
    const resolution = await waitForResolution(resolutions, startIndex);
    exactIdentity(resolution, configuration.expectedIdentity);
    screenshotOrdinal += 1;
    const markerPath = join(outputDir, `pixel-marker-${String(screenshotOrdinal).padStart(6, '0')}.png`);
    await page.screenshot({ path: markerPath, clip: configuration.pixelMarkerRegion });
    if (hashBuffer(readFileSync(markerPath)) !== configuration.expectedIdentity.pixelHash) {
      throw new Error(`${label} remote pixel marker differs from the frozen synthetic fixture`);
    }
    return artifact(markerPath, `p158:${dispatch.runId}:${dispatch.caseId}:pixel:${screenshotOrdinal}`);
  }

  async function captureRegion(action, phase) {
    const path = join(outputDir, `${action.actionId.replaceAll(':', '-')}-${phase}.png`);
    await page.screenshot({ path, clip: action.postcondition.region });
    return artifact(path, `p158:${dispatch.runId}:${action.actionId}:${phase}`);
  }

  async function leaseProjection(contract) {
    const response = await context.request.get(new URL('/api/service/viewer-leases', configuration.handoff.origin).href,
      { failOnStatusCode: false });
    if (!response.ok() || !(response.headers()['content-type'] ?? '').includes('application/json')) {
      throw new Error('Authoritative viewer-lease observation failed');
    }
    const body = await response.json();
    const leases = Array.isArray(body?.viewerLeases) ? body.viewerLeases : [];
    const lease = leases.find((entry) => hashText(String(entry?.id ?? entry?.viewerLeaseId ?? '')) === contract.leaseIdSha256 &&
      (entry?.viewerRole ?? entry?.role) === contract.viewerRole);
    return lease ? {
      leaseIdSha256: contract.leaseIdSha256,
      viewerRole: contract.viewerRole,
      state: lease.state ?? null,
      generation: lease.generation ?? lease.leaseGeneration ?? null,
    } : null;
  }

  return {
    async observeContinuity({ segment, boundary }) {
      const screenshot = await openAndVerify(`segment-${segment.segmentIndex}-${boundary}`);
      return {
        state: 'passed', operatorVisibleState: 'ready', handoffUrlSha256: dispatch.handoffUrlSha256,
        retainedIdentitySha256: dispatch.retainedIdentitySha256, artifacts: [screenshot],
      };
    },
    async observeAction(action) {
      const artifacts = [];
      if (action.kind === 'handoff_reconnect') artifacts.push(await openAndVerify(action.actionId));
      else {
        if (action.postcondition?.kind !== 'pixel_region_transition') {
          throw new Error(`${action.actionId} has no frozen dashboard postcondition`);
        }
        if (!page) await openAndVerify(`${action.actionId}-open`);
        const before = await captureRegion(action, 'before');
        if (before.sha256 !== action.postcondition.beforeSha256) {
          throw new Error(`${action.actionId} pre-action pixels differ from the frozen contract`);
        }
        const frame = page.locator('iframe').first();
        const box = await frame.boundingBox();
        if (!box) throw new Error(`${action.actionId} remote pixel surface is missing`);
        await page.mouse.click(box.x + Math.min(40, box.width / 2), box.y + Math.min(40, box.height / 2));
        await page.keyboard.press('Tab');
        await page.keyboard.press('Escape');
        const after = await captureRegion(action, 'after');
        if (after.sha256 !== action.postcondition.afterSha256) {
          throw new Error(`${action.actionId} post-action pixels do not prove the declared transition`);
        }
        artifacts.push(before, after);
      }
      return {
        actionId: action.actionId, caseId: action.caseId, attemptId: action.attemptId,
        kind: action.kind, state: 'passed', attempt: 1, observedAt: new Date().toISOString(),
        retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
        ...(action.kind === 'dashboard_action' ? {
          postconditionSatisfied: true,
          postconditionSha256: sha256(action.postcondition),
        } : {}),
        artifacts,
      };
    },
    async executeScheduledEvent(event) {
      const artifacts = [];
      let observation;
      if (event.kind === 'scheduled_network_profile') {
        if (event.postcondition?.kind !== 'offline_failure_then_unchanged_handoff_recovery') {
          throw new Error(`${event.eventId} network postcondition is not frozen`);
        }
        await context.setOffline(true);
        let offlineFailed = false;
        try { await page.goto(configuration.handoff.href, { waitUntil: 'domcontentloaded', timeout: 10_000 }); }
        catch { offlineFailed = true; }
        finally { await context.setOffline(false); }
        if (!offlineFailed) throw new Error(`${event.eventId} did not observe the expected offline failure`);
        artifacts.push(await openAndVerify(`${event.eventId}-recovery`));
        observation = { offlineFailureObserved: true, unchangedHandoffRecovered: true,
          handoffUrlSha256: dispatch.handoffUrlSha256, retainedIdentitySha256: dispatch.retainedIdentitySha256 };
      } else if (event.kind === 'client_restart') {
        if (event.postcondition?.kind !== 'retained_identity_reopen' ||
            event.postcondition.retainedIdentitySha256 !== dispatch.retainedIdentitySha256) {
          throw new Error(`${event.eventId} restart identity postcondition is not frozen`);
        }
        await closePage();
        await context.close();
        await browser.close();
        browser = await chromium.launch({ headless: true });
        context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, reducedMotion: 'reduce' });
        await authenticate(context, configuration.handoff, env);
        resolutions = [];
        artifacts.push(await openAndVerify(`${event.eventId}-restart`));
        observation = { clientRestartObserved: true, unchangedHandoffRecovered: true,
          handoffUrlSha256: dispatch.handoffUrlSha256, retainedIdentitySha256: dispatch.retainedIdentitySha256 };
      } else {
        const contract = event.postcondition;
        if (contract?.kind !== 'authoritative_lease_expiry') {
          throw new Error(`${event.eventId} lease-expiry postcondition is not frozen`);
        }
        const before = await leaseProjection(contract);
        if (before?.state !== contract.fromState || before.generation !== contract.baselineGeneration) {
          throw new Error(`${event.eventId} lease did not match the frozen active generation before expiry`);
        }
        await closePage();
        const deadline = Date.now() + contract.timeoutMs;
        let after = null;
        while (Date.now() < deadline) {
          after = await leaseProjection(contract);
          if (after?.state === contract.toState) break;
          await new Promise((resolvePromise) => setTimeout(resolvePromise, 1000));
        }
        if (after?.state !== contract.toState || before.generation === after.generation) {
          throw new Error(`${event.eventId} authoritative lease expiry was not observed`);
        }
        artifacts.push(await openAndVerify(`${event.eventId}-recovery`));
        observation = { before, after, unchangedHandoffRecovered: true,
          handoffUrlSha256: dispatch.handoffUrlSha256, retainedIdentitySha256: dispatch.retainedIdentitySha256 };
      }
      return {
        eventId: event.eventId, kind: event.kind, state: 'passed', attempt: 1,
        observedAt: new Date().toISOString(), retryAttempted: false, repairAttempted: false,
        garbageCollectionAttempted: false, observationSha256: hashText(JSON.stringify(observation)),
        observation, artifacts,
      };
    },
    async close() {
      await closePage();
      await context.close().catch(() => {});
      await browser.close().catch(() => {});
    },
  };
}

async function runShard(args, env) {
  const dispatchPath = resolve(take(args, '--dispatch'));
  const outputDir = resolve(take(args, '--output-dir'));
  const segmentIndex = Number(take(args, '--segment-index'));
  const predecessorPath = take(args, '--predecessor');
  if (args.length) throw new Error(`Unexpected arguments: ${args.join(' ')}`);
  mkdirSync(outputDir, { recursive: true, mode: 0o700 });
  let dispatch = null;
  let driver;
  try {
    dispatch = JSON.parse(readFileSync(dispatchPath, 'utf8'));
    validateP158W9EnduranceDispatch(dispatch);
    const predecessorReceipt = predecessorPath ? JSON.parse(readFileSync(resolve(predecessorPath), 'utf8')) : null;
    driver = await createLiveDriver({ dispatch, outputDir, env });
    let progressOrdinal = 0;
    const receipt = await runP158W9EnduranceShard({
      dispatch, segmentIndex, predecessorReceipt, driver,
      recordProgress: async (entry) => {
        progressOrdinal += 1;
        const body = { progressOrdinal, recordedAt: new Date().toISOString(), ...entry };
        writeFileSync(join(outputDir, `progress-${String(progressOrdinal).padStart(6, '0')}.json`),
          `${JSON.stringify(body, null, 2)}\n`, { mode: 0o600, flag: 'wx' });
      },
      scheduler: { waitUntil: async (value) => {
        const remaining = Date.parse(value) - Date.now();
        if (remaining > 0) await new Promise((resolvePromise) => setTimeout(resolvePromise, remaining));
      } },
    });
    writeFileSync(join(outputDir, 'shard-receipt.json'), `${JSON.stringify(receipt, null, 2)}\n`, { mode: 0o600 });
  } catch (error) {
    writeFileSync(join(outputDir, 'failure-receipt.json'), `${JSON.stringify({
      schemaVersion: 'agent-browser.p158-w9-endurance-failure.v1', planId: 'P158',
      runId: dispatch?.runId ?? env.P158_RUN_ID ?? null, caseId: dispatch?.caseId ?? null,
      dispatchSha256: dispatch?.dispatchSha256 ?? null,
      segmentIndex, failedAt: new Date().toISOString(), failure: { code: error.code ?? 'endurance_shard_failed',
        message: safeMessage(error, env) }, retryAttempted: false, repairAttempted: false,
      garbageCollectionAttempted: false,
    }, null, 2)}\n`, { mode: 0o600 });
    throw error;
  } finally {
    await driver?.close();
  }
}

function bind(args, env) {
  const template = JSON.parse(readFileSync(resolve(take(args, '--template')), 'utf8'));
  const output = resolve(take(args, '--output'));
  if (args.length) throw new Error(`Unexpected arguments: ${args.join(' ')}`);
  if (template.runId !== env.P158_RUN_ID) throw new Error('Endurance template campaign run differs from workflow input');
  for (const field of ['workflow', 'segmentWorkflow', 'runner', 'library',
    'preparationWorkflow', 'preparationRunner', 'preparationLibrary']) {
    const path = template.producer?.[`${field}Path`];
    const expected = template.producer?.[`${field}Sha256`];
    if (typeof path !== 'string' || path.startsWith('/') || path.split('/').includes('..') ||
        hashBuffer(readFileSync(resolve(path))) !== expected) {
      throw new Error(`Endurance producer source changed: ${field}`);
    }
  }
  const dispatch = bindP158W9EnduranceDispatchTemplate({
    template, sourceCommit: env.GITHUB_SHA, workflowRunId: env.GITHUB_RUN_ID,
    workflowRunAttempt: Number(env.GITHUB_RUN_ATTEMPT),
  });
  mkdirSync(resolve(output, '..'), { recursive: true, mode: 0o700 });
  writeFileSync(output, `${JSON.stringify(dispatch, null, 2)}\n`, { mode: 0o600 });
}

function finalize(args) {
  const dispatch = JSON.parse(readFileSync(resolve(take(args, '--dispatch')), 'utf8'));
  const output = resolve(take(args, '--output'));
  const shardPaths = take(args, '--shard', { multiple: true });
  if (args.length) throw new Error(`Unexpected arguments: ${args.join(' ')}`);
  const receipt = finalizeP158W9Endurance({
    dispatch,
    shardReceipts: shardPaths.map((path) => JSON.parse(readFileSync(resolve(path), 'utf8'))),
  });
  mkdirSync(resolve(output, '..'), { recursive: true, mode: 0o700 });
  writeFileSync(output, `${JSON.stringify(receipt, null, 2)}\n`, { mode: 0o600 });
}

function project(args) {
  const dispatch = JSON.parse(readFileSync(resolve(take(args, '--dispatch')), 'utf8'));
  const finalReceipt = JSON.parse(readFileSync(resolve(take(args, '--final-receipt')), 'utf8'));
  const workflowPlan = JSON.parse(readFileSync(resolve(take(args, '--workflow-plan')), 'utf8'));
  const outputDir = resolve(take(args, '--output-dir'));
  if (args.length) throw new Error(`Unexpected arguments: ${args.join(' ')}`);
  const receipts = projectP158W9EnduranceActionReceipts({ dispatch, finalReceipt, workflowPlan });
  mkdirSync(outputDir, { recursive: true, mode: 0o700 });
  for (const receipt of receipts) {
    writeFileSync(join(outputDir, `${receipt.actionId}.json`), `${JSON.stringify(receipt, null, 2)}\n`, { mode: 0o600 });
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2);
  const command = args.shift();
  if (command === 'bind') bind(args, process.env);
  else if (command === 'shard') await runShard(args, process.env);
  else if (command === 'finalize') finalize(args);
  else if (command === 'project') project(args);
  else throw new Error('Usage: run-p158-w9-endurance.js bind|shard|finalize|project ...');
}
