#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { EventEmitter } from 'node:events';
import { readFileSync } from 'node:fs';

import {
  P158_RETAINED_ANCHOR_RECEIPT_SCHEMA,
  runRetainedAuthenticatedAnchor,
  validateRetainedAnchorConfig,
} from './lib/p158-retained-authenticated-anchor.js';
import { createPlaywrightRetainedAnchorAdapter } from './lib/p158-retained-authenticated-anchor-playwright.js';

const secrets = {
  handoffUrl: 'https://public.example.test/remote-view/private-handoff-token',
  username: 'private-anchor-operator',
  password: 'private-anchor-password',
};
const config = {
  runId: 'p158-anchor-test',
  anchorId: 'local-retained-anchor-1',
  ...secrets,
  expectedMarkerSha256: 'a'.repeat(64),
  markerRegion: {
    coordinateSpace: 'remote-view-iframe',
    x: 100,
    y: 100,
    width: 80,
    height: 40,
  },
};
const cleanObservation = {
  authenticatedSession: true,
  markerSha256: config.expectedMarkerSha256,
  iframeCount: 1,
  guacamoleIframe: true,
  streamFailure: false,
  oracleFindingCodes: [],
};

assert.doesNotThrow(() => validateRetainedAnchorConfig(config));
assert.throws(() => validateRetainedAnchorConfig({ ...config, handoffUrl: 'http://127.0.0.1/remote-view/x' }));

let releaseStop;
const stop = new Promise((resolveStop) => { releaseStop = resolveStop; });
const calls = [];
const receipts = [];
const adapter = {
  async open(input) {
    calls.push(['open', input]);
  },
  async observe(input) {
    calls.push(['observe', input]);
    return structuredClone(cleanObservation);
  },
  async close() {
    calls.push(['close']);
  },
};
const run = runRetainedAuthenticatedAnchor({
  config,
  adapter,
  waitForStop: () => stop,
  emitReceipt: async (receipt) => receipts.push(receipt),
  now: (() => {
    const values = ['2026-09-05T12:00:00.000Z', '2026-09-05T12:20:00.000Z'];
    return () => values.shift();
  })(),
});

await new Promise((resolvePromise) => setImmediate(resolvePromise));
assert.equal(receipts.length, 1, 'ready receipt must precede the explicit hold');
assert.equal(calls.filter(([name]) => name === 'open').length, 1);
assert.equal(calls.filter(([name]) => name === 'observe').length, 1);
assert.equal(calls.some(([name]) => name === 'close'), false, 'anchor must remain open during hold');
releaseStop('sigterm');
const result = await run;
assert.equal(receipts.length, 2);
assert.equal(calls.filter(([name]) => name === 'open').length, 1);
assert.equal(calls.filter(([name]) => name === 'observe').length, 2);
assert.equal(calls.filter(([name]) => name === 'close').length, 1);
assert.deepEqual(calls.filter(([name]) => name === 'observe').map(([, input]) => input), [
  { phase: 'ready', waitForConvergence: true },
  { phase: 'final', waitForConvergence: false },
]);
assert.deepEqual(result.readyReceipt, receipts[0]);
assert.deepEqual(result.finalReceipt, receipts[1]);
assert.equal(receipts[0].schemaVersion, P158_RETAINED_ANCHOR_RECEIPT_SCHEMA);
assert.equal(receipts[0].result, 'passed');
assert.equal(receipts[1].result, 'passed');
assert.equal(receipts[1].stopReason, 'sigterm');
for (const receipt of receipts) {
  assert.equal(receipt.maximumNavigationAttempts, 1);
  assert.equal(receipt.retryAttempted, false);
  assert.equal(receipt.repairAttempted, false);
  assert.equal(receipt.reconnectAttempted, false);
  assert.equal(receipt.productActionAttempted, false);
  assert.equal(receipt.evidence.markerMatched, true);
  assert.equal(receipt.evidence.iframeReady, true);
  assert.equal(receipt.evidence.oraclePassed, true);
  const serialized = JSON.stringify(receipt);
  for (const secret of Object.values(secrets)) assert.doesNotMatch(serialized, new RegExp(secret));
}

const failedReceipts = [];
let failedCloseCount = 0;
const privateFailure = new Error(`navigation failed at ${secrets.handoffUrl} using ${secrets.password}`);
await assert.rejects(() => runRetainedAuthenticatedAnchor({
  config,
  adapter: {
    async open() { throw privateFailure; },
    async observe() { throw new Error('must not observe'); },
    async close() { failedCloseCount += 1; },
  },
  waitForStop: async () => { throw new Error('must not hold'); },
  emitReceipt: async (receipt) => failedReceipts.push(receipt),
}));
assert.equal(failedCloseCount, 1, 'a partially opened adapter must still be closed');
assert.equal(failedReceipts.length, 1);
assert.equal(failedReceipts[0].phase, 'ready');
assert.equal(failedReceipts[0].result, 'failed');
assert.equal(failedReceipts[0].failureCode, 'anchor_open_or_readiness_failed');
for (const secret of Object.values(secrets)) {
  assert.doesNotMatch(JSON.stringify(failedReceipts[0]), new RegExp(secret));
}

const finalFailedReceipts = [];
let observationCount = 0;
await assert.rejects(() => runRetainedAuthenticatedAnchor({
  config,
  adapter: {
    async open() {},
    async observe() {
      observationCount += 1;
      return observationCount === 1
        ? structuredClone(cleanObservation)
        : { ...cleanObservation, markerSha256: 'b'.repeat(64) };
    },
    async close() {},
  },
  waitForStop: async () => 'sigint',
  emitReceipt: async (receipt) => finalFailedReceipts.push(receipt),
}));
assert.equal(finalFailedReceipts.length, 2);
assert.equal(finalFailedReceipts[1].phase, 'final');
assert.equal(finalFailedReceipts[1].result, 'failed');
assert.deepEqual(finalFailedReceipts[1].evidence.oracleFindingCodes, ['synthetic_marker_mismatch']);

const markerBytes = Buffer.from('p158-retained-anchor-marker');
const markerDigest = createHash('sha256').update(markerBytes).digest('hex');
let dashboardAuthenticated = true;
let dashboardSessionCookiePresent = true;
let navigationCount = 0;
let authStatusReadCount = 0;
let loginCount = 0;
const fakePage = {
  on() {},
  async goto() {
    navigationCount += 1;
    return { status: () => 200 };
  },
  locator(selector) {
    if (selector === 'iframe') {
      return {
        count: async () => 1,
        first: () => ({
          boundingBox: async () => ({ x: 0, y: 0, width: 1440, height: 1000 }),
          getAttribute: async () => 'https://public.example.test/guacamole/',
        }),
      };
    }
    assert.equal(selector, 'body');
    return { innerText: async () => '' };
  },
  screenshot: async () => markerBytes,
  waitForTimeout: async () => {},
};
const fakeContext = {
  request: {
    async post() {
      loginCount += 1;
      return { ok: () => true };
    },
    async get() {
      authStatusReadCount += 1;
      return {
        ok: () => true,
        json: async () => ({ authenticated: dashboardAuthenticated }),
      };
    },
  },
  async cookies() {
    return [
      { name: 'unrelated_cookie', secure: true, httpOnly: true },
      ...(dashboardSessionCookiePresent
        ? [{ name: 'agent_browser_dashboard_session', secure: true, httpOnly: true }]
        : []),
    ];
  },
  newPage: async () => fakePage,
  close: async () => {},
};
const playwrightAdapter = createPlaywrightRetainedAnchorAdapter({
  chromium: {
    launch: async () => ({
      newContext: async () => fakeContext,
      close: async () => {},
    }),
  },
  convergenceTimeoutMs: 10,
});
const expiryReceipts = [];
await assert.rejects(() => runRetainedAuthenticatedAnchor({
  config: {
    ...config,
    expectedMarkerSha256: markerDigest,
    markerRegion: {
      coordinateSpace: 'viewport',
      x: 10,
      y: 10,
      width: 20,
      height: 20,
    },
  },
  adapter: playwrightAdapter,
  waitForStop: async () => {
    dashboardAuthenticated = false;
    return 'sigterm';
  },
  emitReceipt: async (receipt) => expiryReceipts.push(receipt),
}));
assert.equal(loginCount, 1, 'dashboard login must not be retried after session expiry');
assert.equal(navigationCount, 1, 'session expiry must not navigate or reconnect the handoff');
assert.equal(authStatusReadCount, 3, 'open, ready, and final must each probe current auth status');
assert.equal(expiryReceipts.length, 2);
assert.equal(expiryReceipts[0].result, 'passed');
assert.equal(expiryReceipts[0].evidence.authenticatedSession, true);
assert.equal(expiryReceipts[1].phase, 'final');
assert.equal(expiryReceipts[1].result, 'failed');
assert.equal(expiryReceipts[1].evidence.authenticatedSession, false);
assert.deepEqual(expiryReceipts[1].evidence.oracleFindingCodes, ['authenticated_session_unproven']);
assert.equal(expiryReceipts[1].retryAttempted, false);
assert.equal(expiryReceipts[1].repairAttempted, false);
assert.equal(expiryReceipts[1].reconnectAttempted, false);

dashboardAuthenticated = true;
dashboardSessionCookiePresent = false;
const cookieProofAdapter = createPlaywrightRetainedAnchorAdapter({
  chromium: {
    launch: async () => ({
      newContext: async () => fakeContext,
      close: async () => {},
    }),
  },
  convergenceTimeoutMs: 10,
});
await assert.rejects(
  () => cookieProofAdapter.open(config),
  (error) => error.code === 'dashboard_authenticated_session_unproven',
  'an unrelated secure HttpOnly cookie must not prove a dashboard session',
);
await cookieProofAdapter.close();
assert.equal(navigationCount, 1, 'failed cookie proof must not add a handoff navigation');

for (const signalName of ['SIGTERM', 'SIGINT']) {
  dashboardAuthenticated = true;
  dashboardSessionCookiePresent = true;
  const signalEvents = new EventEmitter();
  const sequence = [];
  let closed = false;
  const explicitStop = new Promise((resolveStop) => {
    signalEvents.once(signalName, () => resolveStop(signalName.toLowerCase()));
  });
  const signalAdapter = createPlaywrightRetainedAnchorAdapter({
    chromium: {
      async launch(options) {
        // Playwright enables automatic process-signal close unless explicitly disabled.
        if (options[signalName === 'SIGTERM' ? 'handleSIGTERM' : 'handleSIGINT'] !== false) {
          signalEvents.once(signalName, () => { closed = true; sequence.push('automatic-close'); });
        }
        return {
          newContext: async () => ({
            ...fakeContext,
            newPage: async () => ({
              ...fakePage,
              screenshot: async () => {
                if (closed) throw new Error('Browser was already closed');
                sequence.push('sample');
                return markerBytes;
              },
            }),
            close: async () => { closed = true; sequence.push('context-close'); },
          }),
          close: async () => sequence.push('browser-close'),
        };
      },
    },
  });
  await runRetainedAuthenticatedAnchor({
    config: { ...config, expectedMarkerSha256: markerDigest },
    adapter: signalAdapter,
    waitForStop: () => {
      sequence.push(signalName);
      signalEvents.emit(signalName);
      return explicitStop;
    },
    emitReceipt: async (receipt) => {
      assert.equal(receipt.result, 'passed');
      sequence.push(`${receipt.phase}-receipt`);
    },
  });
  assert.deepEqual(sequence, ['sample', 'ready-receipt', signalName, 'sample',
    'final-receipt', 'context-close', 'browser-close']);
}

const runnerSource = readFileSync('scripts/run-p158-retained-authenticated-anchor.js', 'utf8');
const adapterSource = readFileSync('scripts/lib/p158-retained-authenticated-anchor-playwright.js', 'utf8');
assert.match(runnerSource, /process\.once\('SIGINT'/);
assert.match(runnerSource, /process\.once\('SIGTERM'/);
assert.doesNotMatch(runnerSource, /setTimeout|retry|repair|reconnect/i);
assert.equal((adapterSource.match(/page\.goto\(/g) ?? []).length, 1);
assert.doesNotMatch(adapterSource, /page\.reload|context\.newPage\(\)[\s\S]*context\.newPage\(\)/);

process.stdout.write('P158 retained authenticated anchor tests passed.\n');
