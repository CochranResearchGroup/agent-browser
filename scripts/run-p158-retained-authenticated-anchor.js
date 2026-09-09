#!/usr/bin/env node

import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

let pendingStopSignal = null;
let resolveStopSignal = null;
const observeStopSignal = (signal) => {
  pendingStopSignal ??= signal;
  resolveStopSignal?.(pendingStopSignal);
};
process.once('SIGINT', () => observeStopSignal('sigint'));
process.once('SIGTERM', () => observeStopSignal('sigterm'));

function parseJsonObject(value, label) {
  try {
    const parsed = JSON.parse(value);
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error();
    return parsed;
  } catch {
    throw new Error(`Invalid ${label} JSON`);
  }
}

function waitForExplicitSignal() {
  if (pendingStopSignal) return Promise.resolve(pendingStopSignal);
  return new Promise((resolveStop) => {
    resolveStopSignal = resolveStop;
  });
}

function receiptWriter(outputDir) {
  return async (receipt) => {
    const path = resolve(outputDir, `${receipt.sequence}-${receipt.phase}-receipt.json`);
    writeFileSync(path, `${JSON.stringify(receipt, null, 2)}\n`, { encoding: 'utf8', mode: 0o600, flag: 'wx' });
    process.stdout.write(`${JSON.stringify({
      phase: receipt.phase,
      result: receipt.result,
      receiptSha256: receipt.receiptSha256,
      outputFile: `${receipt.sequence}-${receipt.phase}-receipt.json`,
    })}\n`);
  };
}

async function main() {
  const [{ chromium }, { runRetainedAuthenticatedAnchor },
    { createPlaywrightRetainedAnchorAdapter }] = await Promise.all([
    import('playwright'),
    import('./lib/p158-retained-authenticated-anchor.js'),
    import('./lib/p158-retained-authenticated-anchor-playwright.js'),
  ]);
  const outputDir = resolve(process.env.P158_ANCHOR_OUTPUT_DIR || 'artifacts/p158-retained-anchor');
  mkdirSync(outputDir, { recursive: true, mode: 0o700 });
  for (const name of ['1-ready-receipt.json', '2-final-receipt.json']) {
    if (existsSync(resolve(outputDir, name))) throw new Error('Anchor receipt destination is not empty');
  }
  const expectedIdentity = parseJsonObject(
    process.env.P158_DEV_EXPECTED_IDENTITY_JSON,
    'expected identity',
  );
  const markerRegion = parseJsonObject(
    process.env.P158_DEV_PIXEL_MARKER_REGION_JSON,
    'pixel marker region',
  );
  await runRetainedAuthenticatedAnchor({
    config: {
      runId: process.env.P158_RUN_ID,
      anchorId: process.env.P158_ANCHOR_ID,
      handoffUrl: process.env.P158_DEV_HANDOFF_URL,
      username: process.env.P158_DEV_DASHBOARD_USERNAME,
      password: process.env.P158_DEV_DASHBOARD_PASSWORD,
      expectedMarkerSha256: expectedIdentity.pixelHash,
      markerRegion,
    },
    adapter: createPlaywrightRetainedAnchorAdapter({ chromium }),
    waitForStop: waitForExplicitSignal,
    emitReceipt: receiptWriter(outputDir),
  });
}

main().catch((error) => {
  const code = typeof error?.code === 'string' && /^[a-z0-9_]+$/.test(error.code)
    ? error.code
    : 'p158_retained_anchor_failed';
  process.stderr.write(`${JSON.stringify({ success: false, failureCode: code })}\n`);
  process.exitCode = 1;
});
